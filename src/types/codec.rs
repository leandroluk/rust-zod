// @spec REQ-PIPE-004
// @spec REQ-PIPE-005
use crate::schema::{
    next_step_id, AsyncOutcome, AsyncStepFn, Aux, Direction, ParseContext, Schema, SchemaDef,
};
use crate::value::Value;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;

fn codec_def<In, Out>(input: &Schema<In>, output: &Schema<Out>) -> SchemaDef {
    SchemaDef::Codec {
        input: input.def.clone(),
        output: output.def.clone(),
    }
}

fn codec_aux<In: 'static, Out: 'static>(input: &Schema<In>, output: &Schema<Out>) -> Aux {
    Aux::Pipe(Arc::new((
        input.clone().into_type::<Value>(),
        output.clone().into_type::<Value>(),
    )))
}

/// Construct a bidirectional codec with explicit input and output schemas.
///
/// `.decode()` / `.parse()` validate with `input`, run `decode`, then validate with `output`.
/// `.encode()` runs the reverse: validate with `output`, run `encode`, then validate with `input`.
/// Failures inside the callbacks are reported with `ctx.issue("...")`.
///
/// The result is a regular `Schema<Out>`, so it composes (inside objects, arrays, unions...) and
/// `.encode()` propagates through the whole tree.
pub fn codec<In: 'static, Out: 'static>(
    input: Schema<In>,
    output: Schema<Out>,
    decode_fn: impl Fn(&Value, &mut ParseContext) -> Option<Value> + Send + Sync + 'static,
    encode_fn: impl Fn(&Value, &mut ParseContext) -> Option<Value> + Send + Sync + 'static,
) -> Schema<Out> {
    let def = codec_def(&input, &output);
    let aux = codec_aux(&input, &output);
    let in_v = input.validator_clone();
    let out_v = output.validator_clone();

    let mut schema = Schema::<Out>::new(def, move |val, ctx| match ctx.direction {
        Direction::Decode => {
            let a = in_v(val, ctx)?;
            let b = decode_fn(&a, ctx)?;
            out_v(&b, ctx)
        }
        Direction::Encode => {
            let a = out_v(val, ctx)?;
            let b = encode_fn(&a, ctx)?;
            in_v(&b, ctx)
        }
    });
    schema.aux = aux;
    schema
}

/// Async codec: `decode` / `encode` return futures (`Ok(value)` or `Err(message)`).
/// Runs under `.decode_async()` / `.encode_async()` / `.parse_async()`; sync entry points
/// return `Failure::Async`. Runtime-agnostic: only `core::future` is used.
pub fn codec_async<In, Out, DF, DFut, EF, EFut>(
    input: Schema<In>,
    output: Schema<Out>,
    decode_fn: DF,
    encode_fn: EF,
) -> Schema<Out>
where
    In: 'static,
    Out: 'static,
    DF: Fn(Value) -> DFut + Send + Sync + 'static,
    DFut: Future<Output = Result<Value, String>> + Send + 'static,
    EF: Fn(Value) -> EFut + Send + Sync + 'static,
    EFut: Future<Output = Result<Value, String>> + Send + 'static,
{
    let def = codec_def(&input, &output);
    let aux = codec_aux(&input, &output);
    let in_v = input.validator_clone();
    let out_v = output.validator_clone();
    let decode_id = next_step_id();
    let encode_id = next_step_id();

    let decode_step: AsyncStepFn = Arc::new(move |v| {
        let fut = decode_fn(v);
        Box::pin(async move {
            match fut.await {
                Ok(v) => AsyncOutcome::Value(v),
                Err(msg) => AsyncOutcome::Fail(msg),
            }
        })
    });
    let encode_step: AsyncStepFn = Arc::new(move |v| {
        let fut = encode_fn(v);
        Box::pin(async move {
            match fut.await {
                Ok(v) => AsyncOutcome::Value(v),
                Err(msg) => AsyncOutcome::Fail(msg),
            }
        })
    });

    let mut schema = Schema::<Out>::new(def, move |val, ctx| {
        let (first, second, id, step) = match ctx.direction {
            Direction::Decode => (&in_v, &out_v, decode_id, &decode_step),
            Direction::Encode => (&out_v, &in_v, encode_id, &encode_step),
        };
        let a = first(val, ctx)?;
        match ctx.async_step(id, &a, step)? {
            AsyncOutcome::Value(b) => second(&b, ctx),
            AsyncOutcome::Fail(msg) => {
                ctx.issue(msg);
                None
            }
            AsyncOutcome::Bool(_) => None,
        }
    });
    schema.aux = aux;
    schema
}

// ----------------------------------------------------------------------------
// Built-in stringbool codec
// ----------------------------------------------------------------------------

#[derive(Clone)]
pub struct StringBoolCodec {
    truthy: Vec<String>,
    falsy: Vec<String>,
    case_sensitive: bool,
}

impl Default for StringBoolCodec {
    fn default() -> Self {
        Self {
            truthy: alloc::vec![
                "true".into(),
                "1".into(),
                "yes".into(),
                "on".into(),
                "y".into(),
                "enabled".into()
            ],
            falsy: alloc::vec![
                "false".into(),
                "0".into(),
                "no".into(),
                "off".into(),
                "n".into(),
                "disabled".into()
            ],
            case_sensitive: false,
        }
    }
}

impl StringBoolCodec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn truthy<const N: usize>(mut self, values: [&str; N]) -> Self {
        self.truthy = values.into_iter().map(String::from).collect();
        self
    }

    pub fn falsy<const N: usize>(mut self, values: [&str; N]) -> Self {
        self.falsy = values.into_iter().map(String::from).collect();
        self
    }

    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    pub fn build(self) -> Schema<bool> {
        let truthy = self.truthy;
        let falsy = self.falsy;
        let case_sensitive = self.case_sensitive;

        let truthy_decode = truthy.clone();
        let falsy_decode = falsy.clone();

        let matches = move |list: &[String], s: &str| {
            list.iter().any(|t| {
                if case_sensitive {
                    t == s
                } else {
                    t.to_lowercase() == s.to_lowercase()
                }
            })
        };

        codec(
            crate::z::string(),
            crate::z::boolean(),
            move |val, ctx| match val {
                Value::String(s) => {
                    if matches(&truthy_decode, s) {
                        Some(Value::Bool(true))
                    } else if matches(&falsy_decode, s) {
                        Some(Value::Bool(false))
                    } else {
                        ctx.add_issue(
                            crate::error::ZodIssueCode::InvalidValue {
                                values: truthy_decode
                                    .iter()
                                    .chain(falsy_decode.iter())
                                    .cloned()
                                    .collect(),
                            },
                            Some("Expected boolean string value"),
                        );
                        None
                    }
                }
                _ => None,
            },
            move |val, _ctx| match val {
                Value::Bool(true) => Some(Value::String(
                    (truthy.first().cloned().unwrap_or_else(|| "true".into())).into(),
                )),
                Value::Bool(false) => Some(Value::String(
                    (falsy.first().cloned().unwrap_or_else(|| "false".into())).into(),
                )),
                _ => None,
            },
        )
    }
}

pub fn stringbool() -> StringBoolCodec {
    StringBoolCodec::new()
}

/// Ready-made codecs for the recipes in the Zod documentation.
pub mod codecs {
    use super::*;
    use crate::error::ZodIssueCode;
    use alloc::format;

    /// `string <-> integer`.
    pub fn string_int() -> Schema<i64> {
        codec(
            crate::z::string(),
            crate::z::int64(),
            |val, ctx| match val {
                Value::String(s) => match s.trim().parse::<i64>() {
                    Ok(i) => Some(Value::Int(i)),
                    Err(_) => {
                        ctx.add_issue(
                            ZodIssueCode::format("integer_string"),
                            Some("Expected valid integer string"),
                        );
                        None
                    }
                },
                _ => None,
            },
            |val, _ctx| match val {
                Value::Int(i) => Some(Value::String(format!("{}", i).into())),
                _ => None,
            },
        )
    }

    /// `string <-> number`.
    pub fn string_number() -> Schema<f64> {
        codec(
            crate::z::string(),
            crate::z::float64(),
            |val, ctx| match val {
                Value::String(s) => match s.trim().parse::<f64>() {
                    Ok(f) if !f.is_nan() && !f.is_infinite() => Some(Value::Float(f)),
                    _ => {
                        ctx.add_issue(
                            ZodIssueCode::format("number_string"),
                            Some("Expected valid number string"),
                        );
                        None
                    }
                },
                _ => None,
            },
            |val, _ctx| match val {
                Value::Float(f) => Some(Value::String(format!("{}", f).into())),
                Value::Int(i) => Some(Value::String(format!("{}", i).into())),
                _ => None,
            },
        )
    }

    /// `epoch milliseconds (integer) <-> date`.
    pub fn epoch_date() -> Schema<i64> {
        codec(
            crate::z::int64(),
            crate::z::date(),
            |val, _ctx| match val {
                Value::Int(ms) => Some(Value::Date(*ms)),
                _ => None,
            },
            |val, _ctx| match val {
                Value::Date(ms) => Some(Value::Int(*ms)),
                _ => None,
            },
        )
    }

    /// `JSON text <-> Value` (parse on decode, stringify on encode).
    #[cfg(feature = "serde_json")]
    pub fn json_string() -> Schema<Value> {
        codec(
            crate::z::string(),
            crate::z::json(),
            |val, ctx| match val {
                Value::String(s) => match serde_json::from_str::<serde_json::Value>(s) {
                    Ok(j) => Some(Value::from(j)),
                    Err(e) => {
                        ctx.issue(format!("Invalid JSON: {}", e));
                        None
                    }
                },
                _ => None,
            },
            |val, _ctx| {
                let j: serde_json::Value = val.clone().into();
                Some(Value::String((j.to_string()).into()))
            },
        )
    }

    /// `base64 text <-> bytes`.
    pub fn base64_bytes() -> Schema<Vec<u8>> {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        fn decode(s: &str) -> Option<Vec<u8>> {
            let mut out = Vec::new();
            let mut buf = 0u32;
            let mut bits = 0;
            for c in s.bytes().filter(|c| *c != b'=') {
                let v = ALPHABET.iter().position(|a| *a == c)? as u32;
                buf = (buf << 6) | v;
                bits += 6;
                if bits >= 8 {
                    bits -= 8;
                    out.push((buf >> bits) as u8);
                    buf &= (1 << bits) - 1;
                }
            }
            Some(out)
        }

        fn encode(bytes: &[u8]) -> String {
            let mut out = String::new();
            for chunk in bytes.chunks(3) {
                let b = [
                    chunk[0],
                    *chunk.get(1).unwrap_or(&0),
                    *chunk.get(2).unwrap_or(&0),
                ];
                let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
                for i in 0..4 {
                    if i <= chunk.len() {
                        out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
                    } else {
                        out.push('=');
                    }
                }
            }
            out
        }

        codec(
            crate::z::base64(),
            crate::types::primitives::bytes(),
            |val, ctx| match val {
                Value::String(s) => match decode(s) {
                    Some(b) => Some(Value::Bytes(b)),
                    None => {
                        ctx.issue("Invalid base64");
                        None
                    }
                },
                _ => None,
            },
            |val, _ctx| match val {
                Value::Bytes(b) => Some(Value::String((encode(b)).into())),
                _ => None,
            },
        )
    }
}
