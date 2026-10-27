// @spec REQ-CORE-001
// @spec REQ-CORE-002
// @spec REQ-CORE-003
// @spec REQ-CORE-004
// @spec REQ-CORE-005
// @spec REQ-CORE-006
// @spec REQ-CORE-007
// @spec REQ-CORE-008
// @spec REQ-CORE-009
// @spec REQ-CORE-010

use rust_zod::z;
use rust_zod::{
    Locale, ParseOptions, PathSegment, Schema, SizeOrigin, Value, ZodError, ZodIssue, ZodIssueCode,
};
use std::collections::BTreeMap;

#[test]
fn test_req_core_001_value_model() {
    // Basic types
    let null_val = Value::from(());
    assert!(null_val.is_null());
    assert_eq!(null_val.type_name(), "null");

    let bool_val = Value::from(true);
    assert_eq!(bool_val.as_bool(), Some(true));
    assert_eq!(bool_val.type_name(), "boolean");

    let int_val = Value::from(42i64);
    assert_eq!(int_val.as_i64(), Some(42));
    assert_eq!(int_val.type_name(), "number");

    let uint_val = Value::from(100u64);
    assert_eq!(uint_val.as_u64(), Some(100));

    let float_val = Value::from(3.5f64);
    assert_eq!(float_val.as_f64(), Some(3.5));

    let string_val = Value::from("rust-zod");
    assert_eq!(string_val.as_str(), Some("rust-zod"));
    assert_eq!(string_val.type_name(), "string");

    let date_val = Value::Date(1700000000000);
    assert_eq!(date_val.as_date(), Some(1700000000000));
    assert_eq!(date_val.type_name(), "date");

    let bytes_val = Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(bytes_val.as_bytes().unwrap().len(), 4);
    assert_eq!(bytes_val.type_name(), "bytes");

    // Array & Object conversions
    let arr = Value::from(vec![Value::from(1), Value::from(2)]);
    assert_eq!(arr.as_array().unwrap().len(), 2);

    let mut map = BTreeMap::new();
    map.insert("name".to_string(), Value::from("Leandro"));
    let obj = Value::from(map);
    assert_eq!(
        obj.as_object().unwrap().get("name").unwrap().as_str(),
        Some("Leandro")
    );

    #[cfg(feature = "serde_json")]
    {
        let json: serde_json::Value = serde_json::json!({
            "status": "ok",
            "code": 200
        });
        let val: Value = json.into();
        assert_eq!(
            val.as_object().unwrap().get("status").unwrap().as_str(),
            Some("ok")
        );
        let back_json: serde_json::Value = val.into();
        assert_eq!(back_json["code"], 200);
    }
}

#[test]
fn test_req_core_002_issue_taxonomy() {
    let issue_codes = vec![
        ZodIssueCode::InvalidType {
            expected: "string".into(),
            received: "number".into(),
        },
        ZodIssueCode::TooBig {
            origin: SizeOrigin::String,
            maximum: 10.0,
            inclusive: true,
            exact: false,
        },
        ZodIssueCode::TooSmall {
            origin: SizeOrigin::Number,
            minimum: 5.0,
            inclusive: false,
            exact: false,
        },
        ZodIssueCode::InvalidFormat {
            format: "email".into(),
            pattern: None,
            prefix: None,
            suffix: None,
            includes: None,
        },
        ZodIssueCode::NotMultipleOf { divisor: 5.0 },
        ZodIssueCode::UnrecognizedKeys {
            keys: vec!["extra".into()],
        },
        ZodIssueCode::InvalidUnion {
            errors: vec![],
            discriminator: None,
            options: vec!["A".into(), "B".into()],
            inclusive: false,
            matches: 0,
        },
        ZodIssueCode::InvalidKey {
            origin: "map".into(),
            issues: vec![],
        },
        ZodIssueCode::InvalidElement {
            origin: "array".into(),
            key: "0".into(),
            issues: vec![],
        },
        ZodIssueCode::InvalidValue {
            values: vec!["admin".into(), "user".into()],
        },
        ZodIssueCode::Custom {
            params: BTreeMap::new(),
        },
    ];

    assert_eq!(
        issue_codes.len(),
        11,
        "Must contain all 11 Zod v4 issue variants"
    );
}

#[test]
fn test_req_core_003_and_004_issues_and_formatters() {
    let issues = vec![
        ZodIssue::new(
            ZodIssueCode::InvalidType {
                expected: "string".into(),
                received: "number".into(),
            },
            vec![
                PathSegment::Key("users".into()),
                PathSegment::Index(0),
                PathSegment::Key("email".into()),
            ],
            "Expected string, received number",
        ),
        ZodIssue::new(
            ZodIssueCode::TooSmall {
                origin: SizeOrigin::Number,
                minimum: 18.0,
                inclusive: true,
                exact: false,
            },
            vec![
                PathSegment::Key("users".into()),
                PathSegment::Index(0),
                PathSegment::Key("age".into()),
            ],
            "Number must be greater than or equal to 18",
        ),
        ZodIssue::new(
            ZodIssueCode::Custom {
                params: BTreeMap::new(),
            },
            vec![],
            "Payload header is missing",
        ),
    ];

    let error = ZodError::new(issues);

    // 1. .flatten()
    let flat = error.flatten();
    assert_eq!(flat.form_errors, vec!["Payload header is missing"]);
    assert!(flat.field_errors.contains_key("users"));

    // 2. .format()
    let formatted = error.format();
    assert_eq!(formatted.errors, vec!["Payload header is missing"]);
    let users = formatted.fields.get("users").unwrap();
    let user_0 = users.fields.get("0").unwrap();
    assert_eq!(
        user_0.fields.get("email").unwrap().errors,
        vec!["Expected string, received number"]
    );

    // 3. .treeify()
    let tree = error.treeify();
    assert_eq!(tree.errors, vec!["Payload header is missing"]);
    let users_prop = tree.properties.get("users").unwrap();
    let user_0_item = users_prop.items.get(&0).unwrap();
    assert!(user_0_item.properties.contains_key("email"));
    assert!(user_0_item.properties.contains_key("age"));

    // 4. .prettify()
    let prettified = error.prettify();
    assert!(prettified.contains("✖ Expected string, received number\n  → at users[0].email"));
    assert!(
        prettified.contains("✖ Number must be greater than or equal to 18\n  → at users[0].age")
    );
    assert!(prettified.contains("✖ Payload header is missing\n  → at <root>"));
}

#[test]
fn test_req_core_005_and_006_i18n_and_default_messages() {
    let schema = z::string();

    // Default English
    z::set_locale(Locale::En);
    let err_en = schema.parse(&Value::from(42)).unwrap_err();
    assert_eq!(err_en.issues[0].message, "Expected string, received number");

    // Global Portuguese
    z::set_locale(Locale::PtBr);
    let err_pt = schema.parse(&Value::from(42)).unwrap_err();
    assert_eq!(err_pt.issues[0].message, "Esperado string, recebido number");

    // Per-parse override Spanish without changing global
    let err_es = schema
        .parse_with(&Value::from(42), ParseOptions::new().locale(Locale::Es))
        .unwrap_err();
    assert_eq!(
        err_es.issues[0].message,
        "Se esperaba string, se recibió number"
    );

    // Global remains PtBr
    assert_eq!(z::get_locale(), Locale::PtBr);

    // Reset back to English
    z::set_locale(Locale::En);
}

#[test]
fn test_req_core_007_metadata_builder() {
    let email_schema = z::string().meta(|m| {
        m.id("UserEmail")
            .title("User Email")
            .description("Primary email address for system notifications")
            .example("user@example.com")
            .deprecated(false)
            .custom("x-security-level", "confidential")
    });

    assert_eq!(email_schema.meta.id.as_deref(), Some("UserEmail"));
    assert_eq!(email_schema.meta.title.as_deref(), Some("User Email"));
    assert_eq!(
        email_schema.meta.description.as_deref(),
        Some("Primary email address for system notifications")
    );
    assert_eq!(email_schema.meta.examples.len(), 1);
    assert_eq!(
        email_schema.meta.custom.get("x-security-level"),
        Some(&Value::from("confidential"))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_req_core_008_sync_and_async_parsing() {
    let number_schema = z::number();

    // Raw value parse
    let raw_val = number_schema.parse_value(&Value::from(100)).unwrap();
    assert_eq!(raw_val.as_i64(), Some(100));

    // Sync parse directly returns strongly typed f64
    let ok_val: f64 = number_schema.parse(&Value::from(100)).unwrap();
    assert_eq!(ok_val, 100.0);

    // No `safe_parse`: `.parse()` returns a Result usable with `?` or `match`
    match number_schema.parse(&Value::from(100)) {
        Ok(v) => assert_eq!(v, 100.0),
        Err(_) => panic!("expected Ok"),
    }
    let err = number_schema
        .parse(&Value::from("not a number"))
        .unwrap_err();
    assert_eq!(err.issues.len(), 1);

    // Async parse
    let async_res = number_schema.parse_async(&Value::from(200)).await;
    assert_eq!(async_res.unwrap(), 200.0);

    let async_err = number_schema.parse_async(&Value::from("bad")).await;
    assert!(async_err.is_err());
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
enum Role {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct UserProfile {
    name: String,
    role: Role,
    tags: Vec<String>,
    bio: Option<String>,
}

#[test]
fn test_section_3_schema_type_conversion_and_parse_into() {
    // 1. z::options typed as Schema<String> and parsed into Rust enum
    let role_schema = z::options(["admin", "user"]);
    let role: Role = role_schema.parse_into(&Value::from("admin")).unwrap();
    assert_eq!(role, Role::Admin);

    // Invalid option fails validation
    let role_err = role_schema.parse(&Value::from("guest"));
    assert!(role_err.is_err());

    // 2. z::array typed as Schema<Vec<T>>
    let tags_schema = z::array(z::string());
    let tags: Vec<String> = tags_schema
        .parse(&Value::from(vec![Value::from("rust"), Value::from("zod")]))
        .unwrap();
    assert_eq!(tags, vec!["rust".to_string(), "zod".to_string()]);

    // 3. Optional modifier
    let opt_string_schema = z::string().optional();
    let some_bio: Option<String> = opt_string_schema.parse(&Value::from("Developer")).unwrap();
    assert_eq!(some_bio, Some("Developer".to_string()));
    let none_bio: Option<String> = opt_string_schema.parse(&Value::Null).unwrap();
    assert_eq!(none_bio, None);

    // 4. Object parsed into typed struct UserProfile (as described in INSIGHT.md §3)
    let mut obj_data = std::collections::BTreeMap::new();
    obj_data.insert("name".to_string(), Value::from("Leandro"));
    obj_data.insert("role".to_string(), Value::from("admin"));
    obj_data.insert(
        "tags".to_string(),
        Value::from(vec![Value::from("rust"), Value::from("zod")]),
    );
    obj_data.insert("bio".to_string(), Value::from("Open source dev"));
    let user_val = Value::from(obj_data);

    let user_schema: Schema<UserProfile> = z::object(|o| {
        o.field("name", z::string().min(2))
            .field("role", z::options(["admin", "user"]))
            .field("tags", z::array(z::string()))
            .field("bio", z::string().optional())
    })
    .into_type::<UserProfile>();

    // Direct .parse() returning typed UserProfile:
    let profile: UserProfile = user_schema.parse(&user_val).unwrap();
    assert_eq!(
        profile,
        UserProfile {
            name: "Leandro".into(),
            role: Role::Admin,
            tags: vec!["rust".into(), "zod".into()],
            bio: Some("Open source dev".into()),
        }
    );

    // Composition with z::array:
    let users_list = Value::from(vec![user_val.clone()]);
    let users: Vec<UserProfile> = z::array(user_schema.clone()).parse(&users_list).unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Leandro");

    // Composition with .optional():
    let maybe_user: Option<UserProfile> = user_schema.clone().optional().parse(&user_val).unwrap();
    assert!(maybe_user.is_some());
    let none_user: Option<UserProfile> = user_schema.optional().parse(&Value::Null).unwrap();
    assert!(none_user.is_none());
}

// ----------------------------------------------------------------------------
// Feature 02 Tests: Primitives, Formats & Coercion
// ----------------------------------------------------------------------------

#[test]
fn test_req_prim_001_primitives() {
    assert!(z::null().parse(&Value::Null).is_ok());
    assert!(z::null().parse(&Value::from(0)).is_err());

    assert!(z::unknown().parse(&Value::from("anything")).is_ok());

    assert!(z::date().parse(&Value::Date(1600000000000)).is_ok());
    assert!(z::date().parse(&Value::from("not a date")).is_err());

    let single_lit = z::literal("open");
    assert!(single_lit.parse(&Value::from("open")).is_ok());
    assert!(single_lit.parse(&Value::from("closed")).is_err());

    let multi_lit = z::literal(["read", "write"]);
    assert!(multi_lit.parse(&Value::from("read")).is_ok());
    assert!(multi_lit.parse(&Value::from("write")).is_ok());
    assert!(multi_lit.parse(&Value::from("execute")).is_err());
}

#[test]
fn test_req_prim_002_string_checks_and_transforms() {
    let schema = z::string().trim().to_lower_case().min(3).max(10);

    // Valid with transforms
    let res = schema.parse(&Value::from("  HELLORUST  ")).unwrap();
    assert_eq!(res.as_str(), "hellorust");

    // Too short
    assert!(schema.parse(&Value::from("hi")).is_err());

    // Starts with, ends with, includes
    let tagged = z::string()
        .starts_with("tag:")
        .ends_with(".rs")
        .includes("zod");
    assert!(tagged.parse(&Value::from("tag:rust-zod.rs")).is_ok());
    assert!(tagged.parse(&Value::from("tag:other.rs")).is_err());

    // Slugify
    let slugger = z::string().slugify();
    let slug = slugger
        .parse(&Value::from("Hello World / Rust Zod v4!"))
        .unwrap();
    assert_eq!(slug.as_str(), "hello-world-rust-zod-v4");
}

#[test]
fn test_req_prim_003_to_006_string_formats_and_iso() {
    // UUID
    let uuid_val = Value::from("123e4567-e89b-12d3-a456-426614174000");
    assert!(z::uuid().parse(&uuid_val).is_ok());
    assert!(z::uuid().parse(&Value::from("not-a-uuid")).is_err());

    // Email
    assert!(z::email().parse(&Value::from("alice@example.com")).is_ok());
    assert!(z::email().parse(&Value::from("invalid-email")).is_err());

    // URL
    assert!(z::url()
        .parse(&Value::from("https://github.com/colinhacks/zod"))
        .is_ok());
    assert!(z::url().parse(&Value::from("invalid-url")).is_err());

    // IPv4 & IPv6
    assert!(z::ipv4().parse(&Value::from("192.168.1.1")).is_ok());
    assert!(z::ipv4().parse(&Value::from("999.1.1.1")).is_err());
    assert!(z::ipv6()
        .parse(&Value::from("2001:0db8:85a3:0000:0000:8a2e:0370:7334"))
        .is_ok());

    // Base64 & Hex
    assert!(z::base64().parse(&Value::from("cm9vdA==")).is_ok());
    assert!(z::hex().parse(&Value::from("deadbeef0123")).is_ok());

    // ISO formats
    assert!(z::iso::date().parse(&Value::from("2026-10-27")).is_ok());
    assert!(z::iso::date().parse(&Value::from("27-10-2026")).is_err());
    assert!(z::iso::datetime()
        .parse(&Value::from("2026-10-27T10:00:00Z"))
        .is_ok());
}

#[test]
fn test_req_prim_007_numeric_types_and_checks() {
    let int_schema = z::int32().min(10).max(100).multiple_of(5);

    assert!(int_schema.parse(&Value::from(25)).is_ok());
    assert!(int_schema.parse(&Value::from(5)).is_err()); // < 10
    assert!(int_schema.parse(&Value::from(105)).is_err()); // > 100
    assert!(int_schema.parse(&Value::from(27)).is_err()); // not multiple of 5

    // Rejection of non-integer for int
    assert!(z::int().parse(&Value::from(3.5)).is_err());

    // Float bounds
    let float_schema = z::float64().positive().lt(1.0);
    assert!(float_schema.parse(&Value::from(0.5)).is_ok());
    assert!(float_schema.parse(&Value::from(0.0)).is_err());
    assert!(float_schema.parse(&Value::from(1.5)).is_err());
}

#[test]
fn test_req_prim_008_coercion() {
    // Coerce to string
    assert_eq!(
        z::coerce::string()
            .parse(&Value::from(42))
            .unwrap()
            .as_str(),
        "42"
    );
    assert_eq!(
        z::coerce::string()
            .parse(&Value::from(true))
            .unwrap()
            .as_str(),
        "true"
    );

    // Coerce to number
    assert_eq!(
        z::coerce::number().parse(&Value::from("42.5")).unwrap(),
        42.5
    );
    assert_eq!(z::coerce::number().parse(&Value::from(true)).unwrap(), 1.0);

    // Coerce to boolean
    assert!(z::coerce::boolean().parse(&Value::from("true")).unwrap());
    assert!(!z::coerce::boolean().parse(&Value::from("off")).unwrap());
    assert!(z::coerce::boolean().parse(&Value::from(1)).unwrap());
}

// ----------------------------------------------------------------------------
// Feature 03 Tests: Options & Collections
// ----------------------------------------------------------------------------

#[test]
fn test_req_coll_001_options_extract_exclude() {
    let role = z::options(["admin", "user", "guest"]);

    assert_eq!(
        role.options_list(),
        Some(vec!["admin".into(), "user".into(), "guest".into()])
    );

    let staff = role.extract(["admin", "user"]);
    assert!(staff.parse(&Value::from("admin")).is_ok());
    assert!(staff.parse(&Value::from("user")).is_ok());
    assert!(staff.parse(&Value::from("guest")).is_err());

    let non_admin = role.exclude(["admin"]);
    assert!(non_admin.parse(&Value::from("user")).is_ok());
    assert!(non_admin.parse(&Value::from("guest")).is_ok());
    assert!(non_admin.parse(&Value::from("admin")).is_err());
}

#[test]
fn test_req_coll_002_array_modifiers() {
    let tags = z::array(z::string()).min(2).max(4).unique();

    // Valid unique array
    assert!(tags
        .parse(&Value::from(vec![Value::from("a"), Value::from("b")]))
        .is_ok());

    // Duplicate element triggers unique error
    assert!(tags
        .parse(&Value::from(vec![Value::from("a"), Value::from("a")]))
        .is_err());

    // Too short (< 2)
    assert!(tags.parse(&Value::from(vec![Value::from("a")])).is_err());

    // Too long (> 4)
    assert!(tags
        .parse(&Value::from(vec![
            Value::from("1"),
            Value::from("2"),
            Value::from("3"),
            Value::from("4"),
            Value::from("5"),
        ]))
        .is_err());
}

#[test]
fn test_req_coll_003_tuple_and_rest() {
    let tuple_schema = z::tuple([
        z::string().into_type::<Value>(),
        z::number().into_type::<Value>(),
    ]);

    assert!(tuple_schema
        .parse(&Value::from(vec![Value::from("hello"), Value::from(42)]))
        .is_ok());
    assert!(tuple_schema
        .parse(&Value::from(vec![Value::from("hello")]))
        .is_err()); // missing second

    // Tuple with rest
    let tuple_with_rest = z::tuple([z::string().into_type::<Value>()]).rest(z::number());

    assert!(tuple_with_rest
        .parse(&Value::from(vec![
            Value::from("key"),
            Value::from(1),
            Value::from(2),
            Value::from(3),
        ]))
        .is_ok());
}

#[test]
fn test_req_coll_004_and_005_set_and_map() {
    // Set enforces unique elements
    let set_schema = z::set(z::string());
    assert!(set_schema
        .parse(&Value::from(vec![Value::from("x"), Value::from("y")]))
        .is_ok());
    assert!(set_schema
        .parse(&Value::from(vec![Value::from("x"), Value::from("x")]))
        .is_err());

    // Map enforces key and value schemas
    let map_schema = z::map(z::string().min(2), z::number());
    let mut obj = std::collections::BTreeMap::new();
    obj.insert("port".to_string(), Value::from(8080));
    assert!(map_schema.parse(&Value::from(obj)).is_ok());

    let mut invalid_key_obj = std::collections::BTreeMap::new();
    invalid_key_obj.insert("p".to_string(), Value::from(8080)); // key length 1 < 2
    assert!(map_schema.parse(&Value::from(invalid_key_obj)).is_err());
}

// ----------------------------------------------------------------------------
// Feature 04 & 05 Tests: Objects, Records, Unions, Intersections & Lazy
// ----------------------------------------------------------------------------

#[test]
fn test_req_obj_001_to_004_objects_and_records() {
    let base_user = z::object(|o| {
        o.field("id", z::uuid())
            .field("name", z::string().min(2))
            .field("email", z::email())
    });

    let mut user_map = std::collections::BTreeMap::new();
    user_map.insert(
        "id".to_string(),
        Value::from("123e4567-e89b-12d3-a456-426614174000"),
    );
    user_map.insert("name".to_string(), Value::from("Alice"));
    user_map.insert("email".to_string(), Value::from("alice@example.com"));
    user_map.insert("extra".to_string(), Value::from("strip_me"));

    // 1. Default strip
    let parsed = base_user
        .clone()
        .parse(&Value::from(user_map.clone()))
        .unwrap();
    let parsed_obj = parsed.as_object().unwrap();
    assert_eq!(parsed_obj.get("name").unwrap().as_str(), Some("Alice"));
    assert!(!parsed_obj.contains_key("extra")); // stripped

    // 2. Strict rejects extra keys
    let strict_schema = base_user.clone().strict();
    assert!(strict_schema.parse(&Value::from(user_map.clone())).is_err());

    // 3. Loose preserves extra keys
    let loose_schema = base_user.clone().loose();
    let loose_parsed = loose_schema.parse(&Value::from(user_map.clone())).unwrap();
    assert!(loose_parsed.as_object().unwrap().contains_key("extra"));

    // 4. Extend
    let extended = base_user
        .clone()
        .extend(|o| o.field("age", z::int32().min(18)));
    user_map.insert("age".to_string(), Value::from(25));
    assert!(extended.parse(&Value::from(user_map.clone())).is_ok());

    // 5. Pick & Omit
    let picked = base_user.pick(["id", "name"]);
    assert!(picked.parse(&Value::from(user_map.clone())).is_ok());

    let omitted = base_user.omit(["email"]);
    assert!(omitted.parse(&Value::from(user_map.clone())).is_ok());

    // 6. Record
    let rec = z::record(z::string(), z::int32());
    let mut rec_map = std::collections::BTreeMap::new();
    rec_map.insert("a".to_string(), Value::from(10));
    rec_map.insert("b".to_string(), Value::from(20));
    assert!(rec.parse(&Value::from(rec_map)).is_ok());
}

#[test]
fn test_req_comb_001_to_005_unions_intersections_lazy() {
    // 1. Union
    let str_or_num = z::union(vec![
        z::string().into_type::<Value>(),
        z::number().into_type::<Value>(),
    ]);
    assert!(str_or_num.parse(&Value::from("hello")).is_ok());
    assert!(str_or_num.parse(&Value::from(42)).is_ok());
    assert!(str_or_num.parse(&Value::from(true)).is_err());

    // 2. XOR (exclusive or)
    let xor_schema = z::xor(vec![
        z::string().min(5).into_type::<Value>(), // matches if len >= 5
        z::string().starts_with("a").into_type::<Value>(), // matches if starts with "a"
    ]);
    // "apple" matches both -> fails XOR!
    assert!(xor_schema.parse(&Value::from("apple")).is_err());
    // "banana" matches len >= 5 only -> succeeds XOR
    assert!(xor_schema.parse(&Value::from("banana")).is_ok());
    // "art" matches starts with "a" only -> succeeds XOR
    assert!(xor_schema.parse(&Value::from("art")).is_ok());

    // 3. Discriminated Union
    let circle = z::object(|o| {
        o.field("kind", z::literal("circle"))
            .field("radius", z::number())
    });

    let square = z::object(|o| {
        o.field("kind", z::literal("square"))
            .field("size", z::number())
    });

    let shape = z::discriminated_union("kind", vec![circle, square]);

    let mut circle_map = std::collections::BTreeMap::new();
    circle_map.insert("kind".to_string(), Value::from("circle"));
    circle_map.insert("radius".to_string(), Value::from(10.0));
    assert!(shape.parse(&Value::from(circle_map)).is_ok());

    let mut unknown_shape = std::collections::BTreeMap::new();
    unknown_shape.insert("kind".to_string(), Value::from("triangle"));
    assert!(shape.parse(&Value::from(unknown_shape)).is_err());

    // 4. Intersection
    let named = z::object(|o| o.field("name", z::string()));
    let aged = z::object(|o| o.field("age", z::int32()));
    let person = z::intersection(named, aged);

    let mut person_map = std::collections::BTreeMap::new();
    person_map.insert("name".to_string(), Value::from("Bob"));
    person_map.insert("age".to_string(), Value::from(30));
    let parsed_person = person.parse(&Value::from(person_map)).unwrap();
    assert_eq!(
        parsed_person
            .as_object()
            .unwrap()
            .get("name")
            .unwrap()
            .as_str(),
        Some("Bob")
    );
    assert_eq!(
        parsed_person
            .as_object()
            .unwrap()
            .get("age")
            .unwrap()
            .as_i64(),
        Some(30)
    );

    // 5. Lazy recursive schema
    fn category_schema() -> Schema<Value> {
        z::lazy(|| {
            z::object(|o| {
                o.field("name", z::string())
                    .field("sub", z::array(category_schema()).optional())
            })
        })
    }

    let cat = category_schema();
    let mut tree = std::collections::BTreeMap::new();
    tree.insert("name".to_string(), Value::from("Electronics"));
    let mut sub_tree = std::collections::BTreeMap::new();
    sub_tree.insert("name".to_string(), Value::from("Laptops"));
    tree.insert("sub".to_string(), Value::from(vec![Value::from(sub_tree)]));

    assert!(cat.parse(&Value::from(tree)).is_ok());
}

// ----------------------------------------------------------------------------
// Feature 06 Tests: Modifiers, Refinements, Pipelines & Codecs
// ----------------------------------------------------------------------------

#[test]
fn test_req_pipe_001_to_003_modifiers_refinements_and_pipelines() {
    // 1. Default & Catch
    let with_default = z::string().default("anonymous");
    assert_eq!(
        with_default.parse(&Value::Null).unwrap().as_str(),
        "anonymous"
    );

    let with_catch = z::number().catch(0.0);
    assert_eq!(with_catch.parse(&Value::from("invalid")).unwrap(), 0.0);

    // 2. Refine
    let even_num = z::int().refine(
        |val| val.as_i64().unwrap_or(1) % 2 == 0,
        "Number must be even",
    );
    assert!(even_num.parse(&Value::from(4)).is_ok());
    assert!(even_num.parse(&Value::from(5)).is_err());

    // 3. Transform & Pipe
    let trimmed_uppercase = z::string().transform(|val| {
        if let Value::String(s) = val {
            Value::from(s.trim().to_uppercase())
        } else {
            val
        }
    });
    let transformed = trimmed_uppercase.parse(&Value::from("  hello  ")).unwrap();
    assert_eq!(transformed.as_str(), Some("HELLO"));

    // Pipe string length into number validator
    let str_len_pipe = z::string()
        .transform(|val| {
            if let Value::String(s) = val {
                Value::from(s.len() as i64)
            } else {
                val
            }
        })
        .pipe(z::int().min(5));

    assert!(str_len_pipe.parse(&Value::from("12345")).is_ok());
    assert!(str_len_pipe.parse(&Value::from("123")).is_err());
}

#[test]
fn test_req_pipe_004_and_005_codecs() {
    // 1. Custom epoch to ISO date string codec
    let epoch_to_iso = z::codec(
        z::int64(),
        z::iso::date(),
        |val, _ctx| val.as_i64().map(|_ts| Value::String("2026-10-27".into())),
        |val, _ctx| val.as_str().map(|_s| Value::Int(1700000000000)),
    );

    // Decode: int -> string ISO
    let decoded = epoch_to_iso.decode(&Value::from(1700000000000i64)).unwrap();
    assert_eq!(decoded, "2026-10-27");

    // Encode: string ISO -> int
    let encoded = epoch_to_iso.encode(&Value::from("2026-10-27")).unwrap();
    assert_eq!(encoded.as_i64(), Some(1700000000000));

    // 2. StringBool codec
    let bool_codec = z::stringbool().build();

    // Decode strings into bool
    assert!(bool_codec.decode(&Value::from("true")).unwrap());
    assert!(bool_codec.decode(&Value::from("yes")).unwrap());
    assert!(bool_codec.decode(&Value::from("1")).unwrap());
    assert!(!bool_codec.decode(&Value::from("false")).unwrap());
    assert!(!bool_codec.decode(&Value::from("off")).unwrap());
    assert!(bool_codec.decode(&Value::from("invalid")).is_err());

    // Encode bool into canonical string
    assert_eq!(
        bool_codec.encode(&Value::from(true)).unwrap().as_str(),
        Some("true")
    );
    assert_eq!(
        bool_codec.encode(&Value::from(false)).unwrap().as_str(),
        Some("false")
    );
}

// ----------------------------------------------------------------------------
// Feature 07 & 08 Tests: JSON Schema, OpenAPI, from_json_schema & GraphQL
// ----------------------------------------------------------------------------

#[test]
fn test_req_exp_001_to_003_json_schema_and_from_json_schema() {
    let user_schema = z::object(|o| {
        o.field("id", z::uuid())
            .field("email", z::email())
            .field("age", z::int32().optional())
    })
    .meta(|m| m.id("User").description("User model"));

    // 1. Export to Draft 2020-12
    let json_schema = z::to_json_schema(&user_schema, rust_zod::JsonSchemaOptions::default());
    let js_obj = json_schema.as_object().unwrap();

    assert_eq!(
        js_obj.get("$schema").unwrap().as_str(),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(js_obj.get("type").unwrap().as_str(), Some("object"));
    assert!(js_obj.contains_key("properties"));
    assert!(js_obj.contains_key("$defs"));

    let props = js_obj.get("properties").unwrap().as_object().unwrap();
    assert!(props.contains_key("id"));
    assert!(props.contains_key("email"));
    assert!(props.contains_key("age"));

    // Required fields (id and email are required; age is optional)
    let required = js_obj.get("required").unwrap().as_array().unwrap();
    assert_eq!(required.len(), 2);

    // 2. OpenAPI 3.0 export
    let openapi_opts =
        rust_zod::JsonSchemaOptions::new().target(rust_zod::JsonSchemaTarget::OpenApi30);
    let openapi_schema = z::to_json_schema(&user_schema, openapi_opts);
    let oas_obj = openapi_schema.as_object().unwrap();
    assert!(oas_obj.contains_key("properties"));

    // 3. Reverse Importer (from_json_schema)
    let imported_schema = z::from_json_schema(&json_schema);
    let mut valid_data = std::collections::BTreeMap::new();
    valid_data.insert(
        "id".to_string(),
        Value::from("123e4567-e89b-12d3-a456-426614174000"),
    );
    valid_data.insert("email".to_string(), Value::from("user@example.com"));

    assert!(imported_schema.parse(&Value::from(valid_data)).is_ok());
}

#[test]
fn test_req_gql_001_to_003_graphql_sdl_generator() {
    let user_schema = z::object(|o| {
        o.field("id", z::uuid())
            .field("name", z::string())
            .field("bio", z::string().optional())
            .field("tags", z::array(z::string()))
    })
    .meta(|m| m.id("User").description("User account entity"));

    // 1. Output type SDL
    let sdl = z::to_graphql_sdl(&user_schema, rust_zod::GraphqlOptions::default());

    // Inverted nullability: id, name, and tags are required (!), bio is optional (no !)
    assert!(sdl.contains("type User {"));
    assert!(sdl.contains("id: ID!"));
    assert!(sdl.contains("name: String!"));
    assert!(sdl.contains("bio: String\n") || sdl.contains("bio: String\r\n"));
    assert!(sdl.contains("tags: [String!]!"));

    // 2. Input type SDL
    let input_opts = rust_zod::GraphqlOptions::new()
        .target(rust_zod::GraphqlTarget::Input)
        .type_name("UserInput");
    let input_sdl = z::to_graphql_sdl(&user_schema, input_opts);
    assert!(input_sdl.contains("input UserInput {"));

    // 3. Enum SDL
    let status_enum = z::options(["ACTIVE", "INACTIVE"]).meta(|m| m.id("UserStatus"));
    let enum_sdl = z::to_graphql_sdl(&status_enum, rust_zod::GraphqlOptions::default());
    assert!(enum_sdl.contains("enum UserStatus {"));
    assert!(enum_sdl.contains("ACTIVE"));
    assert!(enum_sdl.contains("INACTIVE"));
}

#[test]
fn test_req_opt_and_codec_presets() {
    // 1. .nullish(), .unwrap(), .nonoptional()
    let base_schema = z::string().min(2);
    let nullish_schema = base_schema.clone().nullish();
    assert!(nullish_schema.parse(&Value::Null).unwrap().is_none());
    assert_eq!(
        nullish_schema.parse(&Value::from("hello")).unwrap(),
        Some("hello".to_string())
    );

    let unwrapped = nullish_schema.nonoptional();
    assert!(unwrapped.parse(&Value::Null).is_err());
    assert_eq!(unwrapped.parse(&Value::from("hello")).unwrap(), "hello");

    // 2. Fast-path Value target bypass
    let any_schema = z::any();
    let res_val: Value = any_schema.parse(&Value::from("fast-path")).unwrap();
    assert_eq!(res_val, Value::from("fast-path"));

    // 3. z::codecs::string_int
    let str_int = z::codecs::string_int();
    let int_val: i64 = str_int.decode(&Value::from("12345")).unwrap();
    assert_eq!(int_val, 12345);
    let encoded_str: String = str_int
        .encode(&Value::from(12345))
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(encoded_str, "12345");

    // 4. z::codecs::string_number
    let str_num = z::codecs::string_number();
    let num_val: f64 = str_num.decode(&Value::from("42.5")).unwrap();
    assert_eq!(num_val, 42.5);

    // 5. z::codecs::epoch_date
    let epoch_date = z::codecs::epoch_date();
    let date_val = epoch_date
        .decode_value(&Value::from(1700000000000i64))
        .unwrap();
    assert_eq!(date_val, Value::Date(1700000000000));
    let encoded_epoch = epoch_date.encode(&date_val).unwrap();
    assert_eq!(encoded_epoch, Value::Int(1700000000000));
}

// ----------------------------------------------------------------------------
// Audit fixes (SD-1..SD-6) and Feature 09
// ----------------------------------------------------------------------------

fn obj(pairs: &[(&str, Value)]) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.clone());
    }
    Value::from(m)
}

fn category() -> Schema<Value> {
    z::object(|o| {
        o.field("name", z::string())
            .field("sub", z::array(z::lazy(category)).optional())
    })
}

#[test]
fn test_sd2_lazy_is_shared_memoized_and_exports_cycles() {
    // Same capture-free fn => one shared node (OnceLock), so recursion is finite.
    let a = z::lazy(category);
    let b = z::lazy(category);
    match (&*a.def, &*b.def) {
        (rust_zod::SchemaDef::Lazy(x), rust_zod::SchemaDef::Lazy(y)) => {
            assert!(std::sync::Arc::ptr_eq(x, y))
        }
        _ => panic!("expected lazy nodes"),
    }

    let tree = obj(&[
        ("name", Value::from("root")),
        (
            "sub",
            Value::from(vec![obj(&[("name", Value::from("leaf"))])]),
        ),
    ]);
    assert!(z::lazy(category).parse(&tree).is_ok());
    let bad = obj(&[
        ("name", Value::from("root")),
        ("sub", Value::from(vec![obj(&[("name", Value::from(1))])])),
    ]);
    assert!(z::lazy(category).parse(&bad).is_err());

    // JSON Schema: terminates and uses $ref/$defs
    let js = z::to_json_schema(&z::lazy(category), rust_zod::JsonSchemaOptions::default());
    let text = format!("{:?}", js);
    assert!(text.contains("$ref"), "{}", text);
    assert!(js.as_object().unwrap().contains_key("$defs"));

    // GraphQL: terminates and refers to itself by name
    let sdl = z::to_graphql_sdl(
        &z::lazy(category),
        rust_zod::GraphqlOptions::new().type_name("Category"),
    );
    assert!(sdl.contains("type Category {"), "{}", sdl);
    assert!(sdl.contains("sub: [Category!]"), "{}", sdl);
}

#[tokio::test(flavor = "current_thread")]
async fn test_sd3_real_async_steps() {
    let unique_email = z::email().refine_async(
        |v| async move { v.as_str() != Some("taken@example.com") },
        "Email already taken",
    );

    // Sync parse of an async schema is a clean error, never a panic
    let err = unique_email
        .parse(&Value::from("a@example.com"))
        .unwrap_err();
    assert_eq!(err.failure, Some(rust_zod::Failure::Async));
    assert!(err.issues.is_empty());

    assert!(unique_email
        .parse_async(&Value::from("a@example.com"))
        .await
        .is_ok());
    let err = unique_email
        .parse_async(&Value::from("taken@example.com"))
        .await
        .unwrap_err();
    assert_eq!(err.issues[0].message, "Email already taken");

    // Async steps nested in objects keep their path
    let user = z::object(|o| {
        o.field(
            "email",
            z::email().refine_async(|v| async move { v.as_str() != Some("x@x.com") }, "nope"),
        )
    });
    let e = user
        .parse_async(&obj(&[("email", Value::from("x@x.com"))]))
        .await
        .unwrap_err();
    assert_eq!(
        e.issues[0].path,
        vec![rust_zod::PathSegment::Key("email".into())]
    );

    // Chained async steps resolve across replay passes
    let doubled = z::int32()
        .transform_async(|v| async move { Value::Int(v.as_i64().unwrap() * 2) })
        .pipe(z::int32().refine_async(|v| async move { v.as_i64() == Some(42) }, "must be 42"));
    assert_eq!(
        doubled.parse_value_async(&Value::from(21)).await.unwrap(),
        Value::Int(42)
    );
    assert!(doubled.parse_value_async(&Value::from(20)).await.is_err());
    assert_eq!(
        doubled.parse_value(&Value::from(21)).unwrap_err().failure,
        Some(rust_zod::Failure::Async)
    );

    // codec_async: runtime-agnostic, both directions
    let c = z::codec_async(
        z::string(),
        z::int64(),
        |v| async move {
            v.as_str()
                .and_then(|s| s.parse::<i64>().ok())
                .map(Value::Int)
                .ok_or_else(|| "not a number".to_string())
        },
        |v| async move { Ok(Value::from(v.as_i64().unwrap().to_string())) },
    );
    assert_eq!(c.decode_async(&Value::from("7")).await.unwrap(), 7);
    assert_eq!(
        c.encode_async(&Value::from(7)).await.unwrap(),
        Value::from("7")
    );
    assert!(c.decode_async(&Value::from("x")).await.is_err());
}

#[test]
fn test_sd4_input_output_required_and_error_customization() {
    // pipe .input() / .output()
    let p = z::string().pipe(z::string().min(3));
    assert!(p.input().parse_value(&Value::from("a")).is_ok());
    assert!(p.output().parse_value(&Value::from("a")).is_err());

    // .required() undoes optionality
    let o = z::object(|o| o.field("a", z::string().optional()).field("b", z::int32()));
    assert!(o.parse(&obj(&[("b", Value::from(1))])).is_ok());
    let r = o.required();
    assert!(r.parse_value(&obj(&[("b", Value::from(1))])).is_err());
    assert!(r
        .required_keys(["a"])
        .parse_value(&obj(&[("a", Value::from("x")), ("b", Value::from(1))]))
        .is_ok());

    // .error() overrides the schema's own message
    let e = z::email()
        .error("E-mail inválido")
        .parse(&Value::from("nope"))
        .unwrap_err();
    assert_eq!(e.issues[0].message, "E-mail inválido");

    // _with: custom message and abort (later checks are skipped)
    let s = z::string()
        .min_with(8, |e| {
            e.error("A senha deve ter pelo menos 8 caracteres")
                .abort(true)
        })
        .regex("[0-9]");
    let err = s.parse(&Value::from("abc")).unwrap_err();
    assert_eq!(err.issues.len(), 1);
    assert_eq!(
        err.issues[0].message,
        "A senha deve ter pelo menos 8 caracteres"
    );
    let err = s.parse(&Value::from("abcdefghij")).unwrap_err();
    assert!(matches!(
        err.issues[0].code,
        ZodIssueCode::InvalidFormat { .. }
    ));
    assert!(s.parse(&Value::from("abcdefgh1")).is_ok());

    // parse options as closure builder
    let err = z::string()
        .parse_with(&Value::from(1), |o: ParseOptions| o.locale(Locale::PtBr))
        .unwrap_err();
    assert_eq!(err.issues[0].message, "Esperado string, recebido number");
}

#[test]
fn test_sd5_exporters_keep_constraints() {
    let schema = z::object(|o| {
        o.field("email", z::email().max(50))
            .field("name", z::string().min(2).max(20).regex("^[a-z]+$"))
            .field("age", z::int32().gte(18).lt(120))
            .field("tags", z::array(z::string()).min(1).unique())
            .field("pos", z::tuple((z::float64(), z::float64())))
            .field("meta", z::record(z::string(), z::int32()))
            .field("kind", z::literal("a"))
            .field(
                "either",
                z::union([
                    z::string().into_type::<Value>(),
                    z::int32().into_type::<Value>(),
                ]),
            )
            .field("when", z::iso::datetime())
    });
    let js = z::to_json_schema(&schema, rust_zod::JsonSchemaOptions::default());
    let props = js
        .as_object()
        .unwrap()
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap();
    let get = |k: &str| props.get(k).unwrap().as_object().unwrap().clone();

    assert_eq!(get("email").get("format").unwrap().as_str(), Some("email"));
    assert_eq!(get("email").get("maxLength").unwrap().as_i64(), Some(50));
    assert_eq!(get("name").get("minLength").unwrap().as_i64(), Some(2));
    assert_eq!(
        get("name").get("pattern").unwrap().as_str(),
        Some("^[a-z]+$")
    );
    assert_eq!(get("age").get("minimum").unwrap().as_i64(), Some(18));
    assert_eq!(
        get("age").get("exclusiveMaximum").unwrap().as_i64(),
        Some(120)
    );
    assert_eq!(get("age").get("type").unwrap().as_str(), Some("integer"));
    assert_eq!(get("tags").get("minItems").unwrap().as_i64(), Some(1));
    assert_eq!(
        get("tags").get("uniqueItems").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(
        get("pos")
            .get("prefixItems")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(get("meta").contains_key("additionalProperties"));
    assert_eq!(get("kind").get("const").unwrap().as_str(), Some("a"));
    assert_eq!(
        get("either")
            .get("anyOf")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        get("when").get("format").unwrap().as_str(),
        Some("date-time")
    );

    // Draft-07 uses tuple `items` arrays
    let d7 = z::to_json_schema(
        &schema,
        rust_zod::JsonSchemaOptions::new().target(rust_zod::JsonSchemaTarget::Draft07),
    );
    let pos7 = d7
        .as_object()
        .unwrap()
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap()
        .get("pos")
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    assert!(pos7.get("items").unwrap().as_array().is_some());

    // io: Input uses the codec input, Output the codec output
    let c = z::codecs::string_int();
    let input = z::to_json_schema(
        &c,
        rust_zod::JsonSchemaOptions::new().io(rust_zod::SchemaIO::Input),
    );
    let output = z::to_json_schema(&c, rust_zod::JsonSchemaOptions::new());
    assert_eq!(
        input.as_object().unwrap().get("type").unwrap().as_str(),
        Some("string")
    );
    assert_eq!(
        output.as_object().unwrap().get("type").unwrap().as_str(),
        Some("integer")
    );

    // Round trip keeps constraints
    let back = z::from_json_schema(&js);
    let ok = obj(&[
        ("email", Value::from("a@b.co")),
        ("name", Value::from("abc")),
        ("age", Value::from(30)),
        ("tags", Value::from(vec![Value::from("x")])),
        ("pos", Value::from(vec![Value::from(1.0), Value::from(2.0)])),
        ("meta", obj(&[("a", Value::from(1))])),
        ("kind", Value::from("a")),
        ("either", Value::from("s")),
        ("when", Value::from("2026-10-27T10:00:00Z")),
    ]);
    assert!(back.parse_value(&ok).is_ok(), "{:?}", back.parse_value(&ok));
    let mut bad = ok.as_object().unwrap().clone();
    bad.insert("age".into(), Value::from(5));
    assert!(back.parse_value(&Value::from(bad)).is_err());

    // GraphQL: unions, nested objects, scalars
    let gql = z::object(|o| {
        o.field("id", z::uuid())
            .field("created", z::date())
            .field("address", z::object(|a| a.field("street", z::string())))
            .field(
                "shape",
                z::discriminated_union(
                    "kind",
                    [
                        z::object(|c| {
                            c.field("kind", z::literal("circle"))
                                .field("r", z::float64())
                        })
                        .meta(|m| m.id("Circle")),
                        z::object(|c| {
                            c.field("kind", z::literal("square"))
                                .field("side", z::float64())
                        })
                        .meta(|m| m.id("Square")),
                    ],
                ),
            )
    })
    .meta(|m| m.id("Thing"));
    let sdl = z::to_graphql_sdl(&gql, rust_zod::GraphqlOptions::default());
    assert!(sdl.contains("created: DateTime!"), "{}", sdl);
    assert!(sdl.contains("address: ThingAddress!"), "{}", sdl);
    assert!(sdl.contains("type ThingAddress {"), "{}", sdl);
    assert!(
        sdl.contains("union ThingShape = Circle | Square"),
        "{}",
        sdl
    );
    assert!(sdl.contains("scalar DateTime"), "{}", sdl);
}

#[test]
fn test_sd6_encode_propagates_and_one_way_steps_fail() {
    let epoch = z::codecs::epoch_date();

    // object / array / optional propagation
    let schema = z::object(|o| {
        o.field("at", epoch.clone())
            .field("history", z::array(epoch.clone()))
            .field("maybe", epoch.clone().optional())
    });
    let decoded = schema
        .parse_value(&obj(&[
            ("at", Value::Int(1000)),
            ("history", Value::from(vec![Value::Int(1), Value::Int(2)])),
        ]))
        .unwrap();
    assert_eq!(
        decoded.as_object().unwrap().get("at"),
        Some(&Value::Date(1000))
    );

    let encoded = schema.encode(&decoded).unwrap();
    assert_eq!(
        encoded.as_object().unwrap().get("at"),
        Some(&Value::Int(1000))
    );
    assert_eq!(
        encoded.as_object().unwrap().get("history"),
        Some(&Value::from(vec![Value::Int(1), Value::Int(2)]))
    );

    // pipe runs in reverse on encode
    let p = z::string().trim().pipe(z::codecs::string_int());
    assert_eq!(p.encode(&Value::Int(5)).unwrap(), Value::from("5"));

    // checks still run on encode
    let bounded = z::codecs::string_int().gte(10);
    assert!(bounded.encode(&Value::Int(5)).is_err());

    // transform is one-way: encode error, not a ZodIssue, not a panic
    let t = z::string().transform(|v| v);
    let err = t.encode(&Value::from("x")).unwrap_err();
    assert!(err.issues.is_empty());
    assert!(matches!(err.failure, Some(rust_zod::Failure::Encode(_))));
    // ...and it propagates through containers
    let nested = z::array(t);
    assert!(matches!(
        nested
            .encode(&Value::from(vec![Value::from("x")]))
            .unwrap_err()
            .failure,
        Some(rust_zod::Failure::Encode(_))
    ));

    // new ready-made codecs
    let j = z::codecs::json_string();
    assert_eq!(
        j.decode_value(&Value::from("[1,2]")).unwrap(),
        Value::from(vec![Value::Int(1), Value::Int(2)])
    );
    assert_eq!(
        j.encode(&Value::from(vec![Value::Int(1), Value::Int(2)]))
            .unwrap(),
        Value::from("[1,2]")
    );
    let b = z::codecs::base64_bytes();
    let bytes = b.decode_value(&Value::from("aGVsbG8=")).unwrap();
    assert_eq!(bytes, Value::Bytes(b"hello".to_vec()));
    assert_eq!(b.encode(&bytes).unwrap(), Value::from("aGVsbG8="));
}

#[test]
fn test_feature_09_paths_and_fast_paths() {
    // Path stack deeper than the 8-segment inline capacity spills over transparently
    let mut schema = z::string().into_type::<Value>();
    let mut input = Value::from(1);
    let mut expected = Vec::new();
    for i in 0..12 {
        let key = format!("k{}", i);
        let inner = schema.clone();
        schema = z::object(|o| o.field(key.clone(), inner));
        input = obj(&[(key.as_str(), input)]);
        expected.push(rust_zod::PathSegment::Key(key.as_str().into()));
    }
    let err = schema.parse_value(&input).unwrap_err();
    assert_eq!(err.issues.len(), 1);
    assert_eq!(
        err.issues[0].path,
        expected.into_iter().rev().collect::<Vec<_>>()
    );

    // Primitive targets are extracted straight from `Value` (no serde round trip)
    assert_eq!(z::string().parse(&Value::from("x")).unwrap(), "x");
    assert_eq!(z::int32().parse(&Value::from(7)).unwrap(), 7i32);
    assert_eq!(z::uint32().parse(&Value::from(7)).unwrap(), 7u32);
    assert_eq!(z::int64().parse(&Value::from(7)).unwrap(), 7i64);
    assert!(z::boolean().parse(&Value::from(true)).unwrap());
    assert_eq!(z::float64().parse(&Value::from(1.5)).unwrap(), 1.5);
}
