// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A structural checker for the JSON schema subset the AI schemas use, and walkers that check
//! the rules structured output imposes.

use serde_json::Value;

/// Keywords structured output does not support.
pub const UNSUPPORTED: [&str; 7] = [
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minLength",
    "maxLength",
];

/// Checks `value` against `schema`; `root` resolves `#/$defs/...` references.
pub fn validate(value: &Value, schema: &Value, root: &Value) -> Result<(), String> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let name = reference.strip_prefix("#/$defs/").ok_or("unsupported $ref")?;
        let target = root
            .pointer(&format!("/$defs/{name}"))
            .ok_or(format!("missing def {name}"))?;
        return validate(value, target, root);
    }
    if let Some(options) = schema.get("anyOf").and_then(Value::as_array) {
        let errors: Vec<String> = options
            .iter()
            .filter_map(|option| validate(value, option, root).err())
            .collect();
        return if errors.len() < options.len() {
            Ok(())
        } else {
            Err(format!("{value} matches no anyOf branch: {errors:?}"))
        };
    }
    if let Some(constant) = schema.get("const")
        && constant != value
    {
        return Err(format!("{value} is not {constant}"));
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array)
        && !options.contains(value)
    {
        return Err(format!("{value} is not one of {options:?}"));
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => validate_object(value, schema, root),
        Some("array") => {
            let items = value.as_array().ok_or(format!("{value} is not an array"))?;
            let item_schema = schema.get("items").ok_or("array without items")?;
            items.iter().try_for_each(|item| validate(item, item_schema, root))
        }
        Some("string") if !value.is_string() => Err(format!("{value} is not a string")),
        Some("boolean") if !value.is_boolean() => Err(format!("{value} is not a boolean")),
        Some("number") if !value.is_number() => Err(format!("{value} is not a number")),
        Some("integer") if !(value.is_u64() || value.is_i64()) => Err(format!("{value} is not an integer")),
        Some("null") if !value.is_null() => Err(format!("{value} is not null")),
        _ => Ok(()),
    }
}

fn validate_object(value: &Value, schema: &Value, root: &Value) -> Result<(), String> {
    let object = value.as_object().ok_or(format!("{value} is not an object"))?;
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or("object without properties")?;
    for required in schema.get("required").and_then(Value::as_array).into_iter().flatten() {
        let name = required.as_str().unwrap();
        if !object.contains_key(name) {
            return Err(format!("missing required {name} in {value}"));
        }
    }
    for (name, field) in object {
        let field_schema = properties.get(name).ok_or(format!("unexpected property {name}"))?;
        validate(field, field_schema, root).map_err(|error| format!("{name}: {error}"))?;
    }
    Ok(())
}

/// Calls `visit` on every schema object in the tree, `$defs` included.
pub fn walk(schema: &Value, visit: &mut impl FnMut(&Value)) {
    match schema {
        Value::Object(map) => {
            visit(schema);
            for value in map.values() {
                walk(value, visit);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| walk(item, visit)),
        _ => {}
    }
}

/// Every problem that would make structured output reject the schema: an object type that
/// allows extra properties, or an unsupported keyword anywhere in the text.
pub fn structured_output_problems(schema: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    walk(schema, &mut |node| {
        if node.get("type").and_then(Value::as_str) == Some("object")
            && node.get("additionalProperties") != Some(&Value::Bool(false))
        {
            problems.push(format!("object without additionalProperties false: {node}"));
        }
    });
    let text = schema.to_string();
    for keyword in UNSUPPORTED {
        if text.contains(&format!("\"{keyword}\"")) {
            problems.push(format!("unsupported keyword {keyword}"));
        }
    }
    problems
}

/// Names of the `$defs` that reach themselves through `$ref`s, which structured output rejects.
pub fn recursive_defs(schema: &Value) -> Vec<String> {
    let defs = schema
        .get("$defs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let refs_of = |def: &Value| {
        let mut names = Vec::new();
        walk(def, &mut |node| {
            if let Some(name) = node
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"))
            {
                names.push(name.to_owned());
            }
        });
        names
    };
    let mut recursive = Vec::new();
    for start in defs.keys() {
        let mut stack = refs_of(&defs[start]);
        let mut seen = Vec::new();
        while let Some(name) = stack.pop() {
            if &name == start {
                recursive.push(start.clone());
                break;
            }
            if !seen.contains(&name) {
                seen.push(name.clone());
                stack.extend(defs.get(&name).map(refs_of).unwrap_or_default());
            }
        }
    }
    recursive
}
