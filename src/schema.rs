// @spec REQ-CORE-008
// @spec REQ-CORE-009
// @spec REQ-CORE-010
use crate::error::{Failure, PathSegment, ZodError, ZodIssue, ZodIssueCode};
use crate::i18n::{format_issue_message, get_global_locale, Locale};
use crate::meta::{Meta, MetaBuilder};
use crate::types::collections::TupleParts;
use crate::types::object::ObjectShape;
use crate::value::Value;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
#[cfg(feature = "serde_json")]
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use compact_str::CompactString;
use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use smallvec::SmallVec;
use std::sync::{Mutex, OnceLock};

// ----------------------------------------------------------------------------
// AST
// ----------------------------------------------------------------------------

/// A declarative constraint recorded in the AST (used by exporters; enforcement lives in the validator).
#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    /// Lower bound. Counts characters (string), items (array) or the value itself (number).
    Min {
        value: f64,
        inclusive: bool,
    },
    /// Upper bound. Counts characters (string), items (array) or the value itself (number).
    Max {
        value: f64,
        inclusive: bool,
    },
    Length(usize),
    MultipleOf(f64),
    Pattern(String),
    Includes(String),
    StartsWith(String),
    EndsWith(String),
    Lowercase,
    Uppercase,
    Unique,
}

/// Numeric flavour of a `SchemaDef::Number`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumKind {
    Float64,
    Float32,
    Int,
    Int32,
    Uint32,
    Int64,
    Uint64,
}

impl NumKind {
    pub fn is_integer(&self) -> bool {
        !matches!(self, NumKind::Float64 | NumKind::Float32)
    }
}

/// Lazily-resolved schema node. The inner schema is produced once (`OnceLock`) and reused.
pub struct LazyDef {
    init: Box<dyn Fn() -> Schema<Value> + Send + Sync>,
    cell: OnceLock<Schema<Value>>,
}

impl LazyDef {
    pub(crate) fn new(init: impl Fn() -> Schema<Value> + Send + Sync + 'static) -> Self {
        Self {
            init: Box::new(init),
            cell: OnceLock::new(),
        }
    }

    /// Resolve (and memoize) the inner schema.
    pub fn get(&self) -> &Schema<Value> {
        self.cell.get_or_init(|| (self.init)())
    }
}

impl fmt::Debug for LazyDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LazyDef")
    }
}

impl PartialEq for LazyDef {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self, other)
    }
}

/// Introspectable AST Schema Definition.
/// Represents all structural schemas, constraints, and combinators.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaDef {
    Any,
    Never,
    Null,
    Boolean,
    String {
        format: Option<String>,
        checks: Vec<Check>,
    },
    Number {
        kind: NumKind,
        checks: Vec<Check>,
    },
    BigInt,
    Date,
    Bytes,
    Literal(Vec<Value>),
    Array {
        item: Arc<SchemaDef>,
        checks: Vec<Check>,
    },
    Tuple {
        items: Vec<Arc<SchemaDef>>,
        rest: Option<Arc<SchemaDef>>,
    },
    Set(Arc<SchemaDef>),
    Record {
        key: Arc<SchemaDef>,
        value: Arc<SchemaDef>,
    },
    Optional(Arc<SchemaDef>),
    Nullable(Arc<SchemaDef>),
    Default {
        inner: Arc<SchemaDef>,
        value: Value,
    },
    Readonly(Arc<SchemaDef>),
    Object {
        fields: BTreeMap<String, Arc<SchemaDef>>,
        strict: bool,
        loose: bool,
        catchall: Option<Arc<SchemaDef>>,
    },
    Options(Vec<String>),
    Union(Vec<Arc<SchemaDef>>),
    DiscriminatedUnion {
        discriminator: String,
        options: Vec<Arc<SchemaDef>>,
    },
    Xor(Vec<Arc<SchemaDef>>),
    Intersection(Arc<SchemaDef>, Arc<SchemaDef>),
    Lazy(Arc<LazyDef>),
    /// One-way mapping: the output type is unknown to the AST.
    Transform(Arc<SchemaDef>),
    Pipe {
        input: Arc<SchemaDef>,
        output: Arc<SchemaDef>,
    },
    Codec {
        input: Arc<SchemaDef>,
        output: Arc<SchemaDef>,
    },
    /// Metadata wrapper created by `.meta(|m| ...)`.
    Meta {
        inner: Arc<SchemaDef>,
        meta: Arc<Meta>,
    },
    Custom {
        name: String,
    },
}

impl SchemaDef {
    /// Plain `String` definition without a format or checks.
    pub fn string() -> Self {
        SchemaDef::String {
            format: None,
            checks: Vec::new(),
        }
    }

    /// Plain `Number` definition of the given kind without checks.
    pub fn number(kind: NumKind) -> Self {
        SchemaDef::Number {
            kind,
            checks: Vec::new(),
        }
    }

    /// Look through `Meta` wrappers, returning the structural definition.
    pub fn strip_meta(&self) -> &SchemaDef {
        match self {
            SchemaDef::Meta { inner, .. } => inner.strip_meta(),
            other => other,
        }
    }

    /// Outermost metadata attached to this node, if any.
    pub fn meta(&self) -> Option<&Meta> {
        match self {
            SchemaDef::Meta { meta, .. } => Some(meta),
            _ => None,
        }
    }
}

fn push_check_def(def: &mut SchemaDef, check: Check) {
    match def {
        SchemaDef::String { checks, .. }
        | SchemaDef::Number { checks, .. }
        | SchemaDef::Array { checks, .. } => checks.push(check),
        SchemaDef::Meta { inner, .. } => push_check_def(Arc::make_mut(inner), check),
        _ => {}
    }
}

// ----------------------------------------------------------------------------
// Parse context
// ----------------------------------------------------------------------------

/// Direction a schema tree is being walked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// `.parse()` / `.decode()`: input -> output.
    #[default]
    Decode,
    /// `.encode()`: output -> input (codecs and pipes run in reverse).
    Encode,
}

/// Result of an async step, cached between replay passes.
#[derive(Debug, Clone)]
pub(crate) enum AsyncOutcome {
    Bool(bool),
    Value(Value),
    Fail(String),
}

type AsyncFuture = Pin<Box<dyn Future<Output = AsyncOutcome> + Send>>;
pub(crate) type AsyncStepFn = Arc<dyn Fn(Value) -> AsyncFuture + Send + Sync>;

#[derive(Default)]
pub(crate) struct AsyncState {
    results: Vec<(usize, Value, AsyncOutcome)>,
    pending: Vec<(usize, Value, AsyncStepFn)>,
}

pub(crate) type AsyncRuntime = Arc<Mutex<AsyncState>>;

static NEXT_STEP_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) fn next_step_id() -> usize {
    NEXT_STEP_ID.fetch_add(1, Ordering::Relaxed)
}

/// Inline path stack: no heap allocation for paths up to 8 segments deep (spills over transparently).
pub type PathStack = SmallVec<[PathSegment; 8]>;

/// Execution context passed through nested validation steps.
pub struct ParseContext {
    pub path: PathStack,
    pub locale: Locale,
    pub abort_early: bool,
    pub direction: Direction,
    pub issues: Vec<ZodIssue>,
    /// Non-validation failure (async step in a sync parse, one-way step during `.encode()`).
    pub failure: Option<Failure>,
    pub(crate) rt: Option<AsyncRuntime>,
}

impl ParseContext {
    pub fn new(locale: Locale) -> Self {
        Self {
            path: SmallVec::new(),
            locale,
            abort_early: false,
            direction: Direction::Decode,
            issues: Vec::new(),
            failure: None,
            rt: None,
        }
    }

    pub fn with_options(options: ParseOptions) -> Self {
        let mut ctx = Self::new(options.locale.unwrap_or_else(get_global_locale));
        ctx.abort_early = options.abort_early;
        ctx
    }

    /// Fresh sub-context (empty path/issues) inheriting locale, direction and async runtime.
    /// Used by combinators that need to collect issues per branch (unions).
    pub fn fork(&self) -> Self {
        Self {
            path: SmallVec::new(),
            locale: self.locale,
            abort_early: false,
            direction: self.direction,
            issues: Vec::new(),
            failure: None,
            rt: self.rt.clone(),
        }
    }

    /// Propagate a failure raised inside a forked context.
    pub fn absorb_failure(&mut self, sub: &ParseContext) {
        if self.failure.is_none() {
            self.failure = sub.failure.clone();
        }
    }

    pub fn push_key(&mut self, key: impl AsRef<str>) {
        self.path
            .push(PathSegment::Key(CompactString::new(key.as_ref())));
    }

    pub fn push_index(&mut self, index: usize) {
        self.path.push(PathSegment::Index(index));
    }

    pub fn pop(&mut self) -> Option<PathSegment> {
        self.path.pop()
    }

    pub fn add_issue(&mut self, code: ZodIssueCode, custom_message: Option<&str>) {
        let message = match custom_message {
            Some(msg) => msg.to_string(),
            None => format_issue_message(&code, self.locale),
        };
        // Only here does the inline stack become a heap `Vec` (error path only).
        let issue = ZodIssue::new(code, self.path.to_vec(), message);
        self.issues.push(issue);
    }

    /// Report a custom issue (used by refinements and codec callbacks, like `ctx.issues.push` in Zod).
    pub fn issue(&mut self, message: impl AsRef<str>) {
        self.add_issue(
            ZodIssueCode::Custom {
                params: BTreeMap::new(),
            },
            Some(message.as_ref()),
        );
    }

    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    pub fn into_error(self) -> Option<ZodError> {
        if let Some(f) = self.failure {
            return Some(ZodError::from_failure(f));
        }
        if self.issues.is_empty() {
            None
        } else {
            Some(ZodError::new(self.issues))
        }
    }

    /// Resolve (or schedule) an async step. `None` means "suspended/failed": the caller must bail out.
    pub(crate) fn async_step(
        &mut self,
        id: usize,
        input: &Value,
        step: &AsyncStepFn,
    ) -> Option<AsyncOutcome> {
        let rt = match &self.rt {
            Some(rt) => rt.clone(),
            None => {
                self.failure.get_or_insert(Failure::Async);
                return None;
            }
        };
        let mut st = rt.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, _, out)) = st.results.iter().find(|(i, v, _)| *i == id && v == input) {
            return Some(out.clone());
        }
        if !st.pending.iter().any(|(i, v, _)| *i == id && v == input) {
            st.pending.push((id, input.clone(), step.clone()));
        }
        None
    }

    /// Turn a finished pass into the final `Result`.
    fn finish(self, res: Option<Value>) -> Result<Value, ZodError> {
        if let Some(f) = self.failure {
            return Err(ZodError::from_failure(f));
        }
        if !self.issues.is_empty() {
            return Err(ZodError::new(self.issues));
        }
        match res {
            Some(v) => Ok(v),
            None => {
                let mut ctx = ParseContext::new(self.locale);
                ctx.issue("Invalid input");
                Err(ZodError::new(ctx.issues))
            }
        }
    }
}

impl fmt::Debug for ParseContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseContext")
            .field("path", &self.path)
            .field("direction", &self.direction)
            .field("issues", &self.issues)
            .finish()
    }
}

/// Options to customize parsing behavior on a per-call basis.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseOptions {
    pub locale: Option<Locale>,
    pub abort_early: bool,
}

impl ParseOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn locale(mut self, locale: Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    pub fn abort_early(mut self, abort: bool) -> Self {
        self.abort_early = abort;
        self
    }
}

/// Accepts either a `ParseOptions` value or a closure builder `|opt| opt.locale(...)`.
pub trait IntoParseOptions {
    fn into_options(self) -> ParseOptions;
}

impl IntoParseOptions for ParseOptions {
    fn into_options(self) -> ParseOptions {
        self
    }
}

impl<F: FnOnce(ParseOptions) -> ParseOptions> IntoParseOptions for F {
    fn into_options(self) -> ParseOptions {
        self(ParseOptions::default())
    }
}

/// Per-check options: custom message and `abort` (stop running later checks when this one fails).
/// Built with the closure builder: `z::string().min_with(8, |e| e.error("too short").abort(true))`.
#[derive(Debug, Clone, Default)]
pub struct CheckOpts {
    pub(crate) error: Option<String>,
    pub(crate) abort: bool,
}

impl CheckOpts {
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    pub fn abort(mut self, abort: bool) -> Self {
        self.abort = abort;
        self
    }
}

// ----------------------------------------------------------------------------
// Schema<T>
// ----------------------------------------------------------------------------

pub type ValidatorFn = Arc<dyn Fn(&Value, &mut ParseContext) -> Option<Value> + Send + Sync>;

/// Extra runtime structure that cannot live in the (validator-free) AST.
#[derive(Clone, Default)]
pub(crate) enum Aux {
    #[default]
    None,
    Object(Arc<ObjectShape>),
    Tuple(Arc<TupleParts>),
    /// Pipe / codec halves: `(input, output)`.
    Pipe(Arc<(Schema<Value>, Schema<Value>)>),
    /// Wrapped schema (optional, nullable, default, readonly...), for `.unwrap()`.
    Inner(Arc<Schema<Value>>),
}

/// Core Schema abstraction wrapping the AST definition and validation logic.
/// Generic over the phantom output type `T` to provide rich static typing without AST bloat.
pub struct Schema<T = Value> {
    pub def: Arc<SchemaDef>,
    pub meta: Meta,
    pub(crate) validator: ValidatorFn,
    pub(crate) aux: Aux,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Schema<T> {
    fn clone(&self) -> Self {
        Self {
            def: self.def.clone(),
            meta: self.meta.clone(),
            validator: self.validator.clone(),
            aux: self.aux.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Schema<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schema")
            .field("def", &self.def)
            .field("meta", &self.meta)
            .finish()
    }
}

impl<T> Schema<T> {
    pub(crate) fn validator_clone(&self) -> ValidatorFn {
        self.validator.clone()
    }

    pub(crate) fn from_parts(
        def: Arc<SchemaDef>,
        meta: Meta,
        validator: ValidatorFn,
        aux: Aux,
    ) -> Self {
        Self {
            def,
            meta,
            validator,
            aux,
            _marker: PhantomData,
        }
    }

    /// Cast or bind the phantom output type without changing runtime validation AST.
    pub fn into_type<NewT>(self) -> Schema<NewT> {
        Schema::from_parts(self.def, self.meta, self.validator, self.aux)
    }

    /// Clone this schema with a replaced validator (AST, metadata and structure are kept).
    pub(crate) fn with_validator(&self, validator: ValidatorFn) -> Self {
        Self::from_parts(
            self.def.clone(),
            self.meta.clone(),
            validator,
            self.aux.clone(),
        )
    }

    /// Wrapper schema (optional, nullable, ...): new AST node, fresh metadata, inner schema remembered.
    pub(crate) fn wrap<U>(&self, def: SchemaDef, validator: ValidatorFn) -> Schema<U> {
        Schema::from_parts(
            Arc::new(def),
            Meta::default(),
            validator,
            Aux::Inner(Arc::new(self.clone().into_type::<Value>())),
        )
    }
}

impl<T: 'static> Schema<T> {
    pub fn new(
        def: SchemaDef,
        validator: impl Fn(&Value, &mut ParseContext) -> Option<Value> + Send + Sync + 'static,
    ) -> Self {
        Self::from_parts(
            Arc::new(def),
            Meta::default(),
            Arc::new(validator),
            Aux::None,
        )
    }

    /// Attach metadata via the fluent Closure Builder pattern `.meta(|m| ...)`.
    pub fn meta<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(MetaBuilder) -> MetaBuilder,
    {
        let meta = builder_fn(MetaBuilder::new()).build();
        let inner = match &*self.def {
            SchemaDef::Meta { inner, .. } => inner.clone(),
            _ => self.def.clone(),
        };
        self.def = Arc::new(SchemaDef::Meta {
            inner,
            meta: Arc::new(meta.clone()),
        });
        self.meta = meta;
        self
    }

    /// Override the message of every issue this schema itself raises (not those of nested children).
    pub fn error(self, message: impl Into<String>) -> Self {
        let msg = message.into();
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            let start = ctx.issues.len();
            let depth = ctx.path.len();
            let res = inner(val, ctx);
            for issue in ctx.issues[start..].iter_mut() {
                if issue.path.len() == depth {
                    issue.message = msg.clone();
                }
            }
            res
        }))
    }

    /// Register a check: records it in the AST and wraps the validator.
    /// `failure` inspects the validated value and returns the issue to raise, if any.
    pub(crate) fn push_check<F>(mut self, check: Check, opts: CheckOpts, failure: F) -> Self
    where
        F: Fn(&Value) -> Option<(ZodIssueCode, Option<String>)> + Send + Sync + 'static,
    {
        push_check_def(Arc::make_mut(&mut self.def), check);
        let inner = self.validator.clone();
        self.validator = Arc::new(move |val, ctx| {
            // Checks describe the output side: on `.encode()` they run on the incoming value, before the
            // reverse mapping; on decode they run on the mapped result.
            let encoding = ctx.direction == Direction::Encode;
            let res = if encoding {
                None
            } else {
                Some(inner(val, ctx)?)
            };
            let subject = res.as_ref().unwrap_or(val);
            if let Some((code, default_msg)) = failure(subject) {
                let msg = opts.error.as_deref().or(default_msg.as_deref());
                ctx.add_issue(code, msg);
                if opts.abort {
                    return None;
                }
            }
            match res {
                Some(res) => Some(res),
                None => inner(val, ctx),
            }
        });
        self
    }

    // ------------------------------------------------------------------------
    // Wrappers
    // ------------------------------------------------------------------------

    /// Mark this schema as optional, returning a `Schema<Option<T>>` that accepts `Null`.
    pub fn optional(self) -> Schema<Option<T>> {
        let inner_validator = self.validator.clone();
        self.wrap(
            SchemaDef::Optional(self.def.clone()),
            Arc::new(move |val, ctx| {
                if val.is_null() {
                    Some(Value::Null)
                } else {
                    inner_validator(val, ctx)
                }
            }),
        )
    }

    /// Mark this schema as nullable, returning a `Schema<Option<T>>` that accepts `Null`.
    pub fn nullable(self) -> Schema<Option<T>> {
        let inner_validator = self.validator.clone();
        self.wrap(
            SchemaDef::Nullable(self.def.clone()),
            Arc::new(move |val, ctx| {
                if val.is_null() {
                    Some(Value::Null)
                } else {
                    inner_validator(val, ctx)
                }
            }),
        )
    }

    /// Mark this schema as nullish (both optional and nullable).
    pub fn nullish(self) -> Schema<Option<T>> {
        let inner_validator = self.validator.clone();
        self.wrap(
            SchemaDef::Optional(Arc::new(SchemaDef::Nullable(self.def.clone()))),
            Arc::new(move |val, ctx| {
                if val.is_null() {
                    Some(Value::Null)
                } else {
                    inner_validator(val, ctx)
                }
            }),
        )
    }

    /// Unwrap an optional / nullable / default / readonly schema, returning the inner schema.
    /// Schemas that wrap nothing are returned unchanged.
    pub fn unwrap(self) -> Schema<Value> {
        match &self.aux {
            Aux::Inner(inner) => (**inner).clone(),
            _ => self.into_type::<Value>(),
        }
    }

    /// Fallback default value if input is `Value::Null`.
    pub fn default(self, default_val: impl Into<Value>) -> Self {
        let default_v: Value = default_val.into();
        let inner = self.validator.clone();
        self.wrap(
            SchemaDef::Default {
                inner: self.def.clone(),
                value: default_v.clone(),
            },
            Arc::new(move |val, ctx| {
                if val.is_null() {
                    Some(default_v.clone())
                } else {
                    inner(val, ctx)
                }
            }),
        )
    }

    /// Prefault value replaces `Value::Null` before running any validations.
    pub fn prefault(self, prefault_val: impl Into<Value>) -> Self {
        let prefault_v = prefault_val.into();
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            let actual = if val.is_null() { &prefault_v } else { val };
            inner(actual, ctx)
        }))
    }

    /// Catch validation errors and return a fallback value instead.
    pub fn catch(self, fallback: impl Into<Value>) -> Self {
        let fallback_v = fallback.into();
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            let initial_issues_len = ctx.issues.len();
            let res = inner(val, ctx);
            if ctx.issues.len() > initial_issues_len || res.is_none() {
                ctx.issues.truncate(initial_issues_len);
                Some(fallback_v.clone())
            } else {
                res
            }
        }))
    }

    /// Mark schema as readonly (exported as `readOnly: true`).
    pub fn readonly(self) -> Self {
        let inner = self.validator.clone();
        self.wrap(SchemaDef::Readonly(self.def.clone()), inner)
    }

    // ------------------------------------------------------------------------
    // Refinements & transforms
    // ------------------------------------------------------------------------

    /// Add a custom validation predicate with a custom error message.
    pub fn refine<F>(self, predicate: F, message: impl Into<String>) -> Self
    where
        F: Fn(&Value) -> bool + Send + Sync + 'static,
    {
        let msg = message.into();
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            if ctx.direction == Direction::Encode {
                if !predicate(val) {
                    ctx.issue(&msg);
                }
                return inner(val, ctx);
            }
            let res = inner(val, ctx)?;
            if !predicate(&res) {
                ctx.issue(&msg);
            }
            Some(res)
        }))
    }

    /// Async refinement. Only runs under `parse_async` / `decode_async` / `encode_async`;
    /// a synchronous parse of such a schema returns `Failure::Async` instead of panicking.
    pub fn refine_async<F, Fut>(self, predicate: F, message: impl Into<String>) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        let msg = message.into();
        let id = next_step_id();
        let step: AsyncStepFn = Arc::new(move |v| {
            let fut = predicate(v);
            Box::pin(async move { AsyncOutcome::Bool(fut.await) })
        });
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            if ctx.direction == Direction::Encode {
                if let AsyncOutcome::Bool(false) = ctx.async_step(id, val, &step)? {
                    ctx.issue(&msg);
                }
                return inner(val, ctx);
            }
            let res = inner(val, ctx)?;
            if let AsyncOutcome::Bool(false) = ctx.async_step(id, &res, &step)? {
                ctx.issue(&msg);
            }
            Some(res)
        }))
    }

    /// Add a fine-grained refinement hook with direct access to `ParseContext`.
    pub fn super_refine<F>(self, refinement_fn: F) -> Self
    where
        F: Fn(&Value, &mut ParseContext) + Send + Sync + 'static,
    {
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            if ctx.direction == Direction::Encode {
                refinement_fn(val, ctx);
                return inner(val, ctx);
            }
            let res = inner(val, ctx)?;
            refinement_fn(&res, ctx);
            Some(res)
        }))
    }

    /// Add a lightweight check assertion.
    pub fn check<F>(self, check_fn: F) -> Self
    where
        F: Fn(&Value, &mut ParseContext) + Send + Sync + 'static,
    {
        self.super_refine(check_fn)
    }

    /// Unidirectional transformation. Reaching it through `.encode()` is an encode failure.
    pub fn transform<F>(self, transform_fn: F) -> Schema<Value>
    where
        F: Fn(Value) -> Value + Send + Sync + 'static,
    {
        let inner = self.validator.clone();
        self.wrap(
            SchemaDef::Transform(self.def.clone()),
            Arc::new(move |val, ctx| {
                if ctx.direction == Direction::Encode {
                    ctx.failure.get_or_insert(Failure::Encode(
                        "`.transform()` is one-way and cannot be encoded".into(),
                    ));
                    return None;
                }
                let res = inner(val, ctx)?;
                Some(transform_fn(res))
            }),
        )
    }

    /// Async unidirectional transformation (see `refine_async`).
    pub fn transform_async<F, Fut>(self, transform_fn: F) -> Schema<Value>
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Value> + Send + 'static,
    {
        let id = next_step_id();
        let step: AsyncStepFn = Arc::new(move |v| {
            let fut = transform_fn(v);
            Box::pin(async move { AsyncOutcome::Value(fut.await) })
        });
        let inner = self.validator.clone();
        self.wrap(
            SchemaDef::Transform(self.def.clone()),
            Arc::new(move |val, ctx| {
                if ctx.direction == Direction::Encode {
                    ctx.failure.get_or_insert(Failure::Encode(
                        "`.transform_async()` is one-way and cannot be encoded".into(),
                    ));
                    return None;
                }
                let res = inner(val, ctx)?;
                match ctx.async_step(id, &res, &step)? {
                    AsyncOutcome::Value(v) => Some(v),
                    _ => None,
                }
            }),
        )
    }

    /// Preprocess raw input value before schema validation begins (one-way).
    pub fn preprocess<F>(self, preprocess_fn: F) -> Self
    where
        F: Fn(&Value) -> Value + Send + Sync + 'static,
    {
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            if ctx.direction == Direction::Encode {
                ctx.failure.get_or_insert(Failure::Encode(
                    "`.preprocess()` is one-way and cannot be encoded".into(),
                ));
                return None;
            }
            let transformed = preprocess_fn(val);
            inner(&transformed, ctx)
        }))
    }

    /// Pipe output of this schema into a target schema. Reversed on `.encode()`.
    pub fn pipe<NextT: 'static>(self, target: Schema<NextT>) -> Schema<NextT> {
        let first_val = self.validator.clone();
        let second_val = target.validator.clone();
        let def = SchemaDef::Pipe {
            input: self.def.clone(),
            output: target.def.clone(),
        };
        let aux = Aux::Pipe(Arc::new((
            self.into_type::<Value>(),
            target.clone().into_type::<Value>(),
        )));

        Schema::from_parts(
            Arc::new(def),
            target.meta.clone(),
            Arc::new(move |val, ctx| match ctx.direction {
                Direction::Decode => {
                    let inter = first_val(val, ctx)?;
                    second_val(&inter, ctx)
                }
                Direction::Encode => {
                    let inter = second_val(val, ctx)?;
                    first_val(&inter, ctx)
                }
            }),
            aux,
        )
    }

    /// Input half of a pipe/codec (`pipe.in` in Zod). Other schemas return themselves.
    pub fn input(&self) -> Schema<Value> {
        match &self.aux {
            Aux::Pipe(parts) => parts.0.clone(),
            _ => self.clone().into_type::<Value>(),
        }
    }

    /// Output half of a pipe/codec (`pipe.out` in Zod). Other schemas return themselves.
    pub fn output(&self) -> Schema<Value> {
        match &self.aux {
            Aux::Pipe(parts) => parts.1.clone(),
            _ => self.clone().into_type::<Value>(),
        }
    }
}

impl<T: 'static> Schema<Option<T>> {
    /// Remove optionality: the inner schema, typed.
    pub fn nonoptional(self) -> Schema<T> {
        self.unwrap().into_type::<T>()
    }
}

// ----------------------------------------------------------------------------
// Parsing
// ----------------------------------------------------------------------------

impl<T: 'static> Schema<T> {
    fn run(
        &self,
        input: &Value,
        options: ParseOptions,
        direction: Direction,
    ) -> Result<Value, ZodError> {
        let mut ctx = ParseContext::with_options(options);
        ctx.direction = direction;
        let res = (self.validator)(input, &mut ctx);
        ctx.finish(res)
    }

    async fn run_async(
        &self,
        input: &Value,
        options: ParseOptions,
        direction: Direction,
    ) -> Result<Value, ZodError> {
        let state: AsyncRuntime = Arc::new(Mutex::new(AsyncState::default()));
        loop {
            let mut ctx = ParseContext::with_options(options);
            ctx.direction = direction;
            ctx.rt = Some(state.clone());
            let res = (self.validator)(input, &mut ctx);

            let pending = {
                let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                core::mem::take(&mut st.pending)
            };
            if pending.is_empty() || ctx.failure.is_some() {
                return ctx.finish(res);
            }
            drop(ctx);

            for (id, val, step) in pending {
                let outcome = step(val.clone()).await;
                state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .results
                    .push((id, val, outcome));
            }
        }
    }

    /// Parse into the raw validated `Value`.
    pub fn parse_value(&self, input: &Value) -> Result<Value, ZodError> {
        self.run(input, ParseOptions::default(), Direction::Decode)
    }

    /// Parse into the raw validated `Value` with explicit options (`ParseOptions` or `|o| o.locale(..)`).
    pub fn parse_value_with(
        &self,
        input: &Value,
        options: impl IntoParseOptions,
    ) -> Result<Value, ZodError> {
        self.run(input, options.into_options(), Direction::Decode)
    }

    /// Async parse into the raw validated `Value` (runs `refine_async` / `transform_async` steps).
    pub async fn parse_value_async(&self, input: &Value) -> Result<Value, ZodError> {
        self.run_async(input, ParseOptions::default(), Direction::Decode)
            .await
    }

    /// Run the schema in reverse (output -> input), returning the raw `Value`.
    pub fn encode(&self, input: &Value) -> Result<Value, ZodError> {
        self.run(input, ParseOptions::default(), Direction::Encode)
    }

    pub fn encode_with(
        &self,
        input: &Value,
        options: impl IntoParseOptions,
    ) -> Result<Value, ZodError> {
        self.run(input, options.into_options(), Direction::Encode)
    }

    pub async fn encode_async(&self, input: &Value) -> Result<Value, ZodError> {
        self.run_async(input, ParseOptions::default(), Direction::Encode)
            .await
    }

    /// `.decode()` is `.parse()` (kept for symmetry with `.encode()`), returning the raw `Value`.
    pub fn decode_value(&self, input: &Value) -> Result<Value, ZodError> {
        self.parse_value(input)
    }

    pub async fn decode_value_async(&self, input: &Value) -> Result<Value, ZodError> {
        self.parse_value_async(input).await
    }
}

#[cfg(feature = "serde_json")]
fn downcast<A: 'static, B: 'static>(a: A) -> B {
    let boxed: Box<dyn core::any::Any> = Box::new(a);
    match boxed.downcast::<B>() {
        Ok(b) => *b,
        Err(_) => unreachable!("TypeId matched before downcast"),
    }
}

/// Direct extraction for `Value` and primitive targets, bypassing serde entirely (zero intermediate JSON).
#[cfg(feature = "serde_json")]
fn fast_path<Target: 'static>(v: Value) -> Result<Target, Value> {
    use core::any::TypeId;
    let id = TypeId::of::<Target>();
    if id == TypeId::of::<Value>() {
        return Ok(downcast(v));
    }
    if id == TypeId::of::<String>() {
        return match v {
            Value::String(s) => Ok(downcast(s.to_string())),
            other => Err(other),
        };
    }
    if id == TypeId::of::<bool>() {
        return match v.as_bool() {
            Some(b) => Ok(downcast(b)),
            None => Err(v),
        };
    }
    if id == TypeId::of::<f64>() {
        return match v.as_f64() {
            Some(x) => Ok(downcast(x)),
            None => Err(v),
        };
    }
    if id == TypeId::of::<f32>() {
        return match v.as_f64() {
            Some(x) => Ok(downcast(x as f32)),
            None => Err(v),
        };
    }
    if id == TypeId::of::<i64>() {
        return match v.as_i64() {
            Some(x) => Ok(downcast(x)),
            None => Err(v),
        };
    }
    if id == TypeId::of::<u64>() {
        return match v.as_u64() {
            Some(x) => Ok(downcast(x)),
            None => Err(v),
        };
    }
    if id == TypeId::of::<i32>() {
        return match v.as_i64().and_then(|x| i32::try_from(x).ok()) {
            Some(x) => Ok(downcast(x)),
            None => Err(v),
        };
    }
    if id == TypeId::of::<u32>() {
        return match v.as_u64().and_then(|x| u32::try_from(x).ok()) {
            Some(x) => Ok(downcast(x)),
            None => Err(v),
        };
    }
    Err(v)
}

#[cfg(feature = "serde_json")]
fn convert<Target: serde::de::DeserializeOwned + 'static>(
    validated: Value,
) -> Result<Target, ZodError> {
    let validated = match fast_path::<Target>(validated) {
        Ok(t) => return Ok(t),
        Err(v) => v,
    };
    let json_val: serde_json::Value = validated.into();
    serde_json::from_value::<Target>(json_val).map_err(|e| {
        let mut params = BTreeMap::new();
        params.insert("serde_error".into(), Value::String((e.to_string()).into()));
        ZodError::from_issue(ZodIssue::new(
            ZodIssueCode::Custom { params },
            Vec::new(),
            format!("Deserialization error into target type: {}", e),
        ))
    })
}

#[cfg(feature = "serde_json")]
impl<T: 'static> Schema<T> {
    /// Synchronously parse and deserialize into the schema's strongly typed output `T`.
    pub fn parse(&self, input: &Value) -> Result<T, ZodError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.parse_into::<T>(input)
    }

    pub fn parse_with(&self, input: &Value, options: impl IntoParseOptions) -> Result<T, ZodError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.parse_into_with::<T>(input, options)
    }

    pub async fn parse_async(&self, input: &Value) -> Result<T, ZodError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.parse_into_async::<T>(input).await
    }

    /// `.decode()` is `.parse()` (symmetry with `.encode()`).
    pub fn decode(&self, input: &Value) -> Result<T, ZodError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.parse(input)
    }

    pub async fn decode_async(&self, input: &Value) -> Result<T, ZodError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.parse_async(input).await
    }

    /// Universal deserialization from validated Value into any strongly typed Rust struct/enum `Target`.
    pub fn parse_into<Target: serde::de::DeserializeOwned + 'static>(
        &self,
        input: &Value,
    ) -> Result<Target, ZodError> {
        convert::<Target>(self.parse_value(input)?)
    }

    pub fn parse_into_with<Target: serde::de::DeserializeOwned + 'static>(
        &self,
        input: &Value,
        options: impl IntoParseOptions,
    ) -> Result<Target, ZodError> {
        convert::<Target>(self.parse_value_with(input, options)?)
    }

    pub async fn parse_into_async<Target: serde::de::DeserializeOwned + 'static>(
        &self,
        input: &Value,
    ) -> Result<Target, ZodError> {
        convert::<Target>(self.parse_value_async(input).await?)
    }

    pub fn decode_into<Target: serde::de::DeserializeOwned + 'static>(
        &self,
        input: &Value,
    ) -> Result<Target, ZodError> {
        self.parse_into::<Target>(input)
    }

    /// Encode, then deserialize the result (the schema's *input* side) into `Target`.
    pub fn encode_into<Target: serde::de::DeserializeOwned + 'static>(
        &self,
        input: &Value,
    ) -> Result<Target, ZodError> {
        convert::<Target>(self.encode(input)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_execution_and_meta() {
        let string_schema = Schema::<String>::new(SchemaDef::string(), |val, ctx| match val {
            Value::String(s) => Some(Value::String(s.clone())),
            _ => {
                ctx.add_issue(
                    ZodIssueCode::InvalidType {
                        expected: "string".into(),
                        received: val.type_name().into(),
                    },
                    None,
                );
                None
            }
        })
        .meta(|m| m.id("TestString").description("A test string"));

        assert_eq!(string_schema.meta.id.as_deref(), Some("TestString"));

        // Valid parse
        let res = string_schema.parse(&Value::from("hello"));
        assert!(res.is_ok());

        // Optional modifier
        let opt_schema = string_schema.clone().optional();
        assert!(opt_schema.parse(&Value::Null).is_ok());

        // Invalid parse
        let err = string_schema.parse(&Value::from(123)).unwrap_err();
        assert_eq!(err.issues.len(), 1);
        assert_eq!(err.issues[0].message, "Expected string, received number");

        // Parse with custom locale (struct and closure builder)
        let err_pt = string_schema
            .parse_with(&Value::from(123), ParseOptions::new().locale(Locale::PtBr))
            .unwrap_err();
        assert_eq!(err_pt.issues[0].message, "Esperado string, recebido number");
        let err_pt2 = string_schema
            .parse_with(&Value::from(123), |o: ParseOptions| o.locale(Locale::PtBr))
            .unwrap_err();
        assert_eq!(
            err_pt2.issues[0].message,
            "Esperado string, recebido number"
        );
    }
}
