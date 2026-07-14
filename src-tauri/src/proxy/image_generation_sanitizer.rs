//! Removes Codex image-generation capability declarations before forwarding a
//! native Responses request to a third-party provider.

use serde_json::Value;

const IMAGE_GENERATION_TYPES: [&str; 3] = [
    "image_generation",
    "image_generation_call",
    "image_generation_preview",
];

pub(crate) fn is_responses_endpoint(endpoint: &str) -> bool {
    let path = endpoint
        .split_once('?')
        .map_or(endpoint, |(path, _query)| path);
    matches!(path, "/responses" | "/v1/responses")
}

pub(crate) fn strip_image_generation_items(body: Value) -> (Value, usize) {
    let mut removed = 0;
    let sanitized = strip_recursive(body, &mut removed);
    (sanitized, removed)
}

fn strip_recursive(value: Value, removed: &mut usize) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .filter_map(|item| {
                    if should_drop_object(&item) {
                        *removed += 1;
                        None
                    } else {
                        Some(strip_recursive(item, removed))
                    }
                })
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter_map(|(key, child)| {
                    if should_drop_object(&child) {
                        *removed += 1;
                        None
                    } else {
                        Some((key, strip_recursive(child, removed)))
                    }
                })
                .collect(),
        ),
        other => other,
    }
}

fn should_drop_object(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    if object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| IMAGE_GENERATION_TYPES.contains(&kind))
    {
        return true;
    }

    object
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(is_image_generation_name)
}

fn is_image_generation_name(name: &str) -> bool {
    let normalized: String = name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    // Recent Codex Desktop builds advertise image generation as an `image_gen`
    // namespace containing an `imagegen` function. Older builds used names such
    // as `image_generation` and `internal_image_generation_preview`.
    normalized == "imagegen" || normalized.contains("imagegeneration")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recognizes_responses_paths_with_optional_query() {
        assert!(is_responses_endpoint("/responses"));
        assert!(is_responses_endpoint("/v1/responses?beta=true"));
        assert!(!is_responses_endpoint("/responses/compact"));
        assert!(!is_responses_endpoint("/v1/chat/completions"));
    }

    #[test]
    fn recursively_removes_image_generation_items_and_preserves_other_tools() {
        let input = json!({
            "model": "gpt-5",
            "tools": [
                {"type": "function", "name": "apply_patch"},
                {"type": "image_generation"},
                {"type": "function", "name": "internal_IMAGE_GENERATION_preview"}
            ],
            "input": [
                {"type": "message", "content": [{"type": "input_text", "text": "hello"}]},
                {"type": "image_generation_call", "id": "img_123"},
                {"metadata": {"preview": {"type": "image_generation_preview"}}}
            ]
        });

        let (output, removed) = strip_image_generation_items(input);

        assert_eq!(removed, 4);
        assert_eq!(
            output["tools"],
            json!([{"type": "function", "name": "apply_patch"}])
        );
        assert_eq!(output["input"].as_array().unwrap().len(), 2);
        assert!(output["input"][1].get("metadata").is_some());
        assert!(output["input"][1]["metadata"].get("preview").is_none());
    }

    #[test]
    fn preserves_input_images_and_unrelated_names() {
        let input = json!({
            "input": [{"type": "input_image", "image_url": "data:image/png;base64,abc"}],
            "tools": [{"type": "function", "name": "generate_image_caption"}]
        });

        let (output, removed) = strip_image_generation_items(input.clone());

        assert_eq!(removed, 0);
        assert_eq!(output, input);
    }

    #[test]
    fn removes_current_codex_image_gen_namespace_and_function() {
        let input = json!({
            "tools": [
                {
                    "type": "namespace",
                    "name": "image_gen",
                    "tools": [{"type": "function", "name": "imagegen"}]
                },
                {"type": "function", "name": "imageGen"},
                {"type": "function", "name": "generate_image_caption"}
            ]
        });

        let (output, removed) = strip_image_generation_items(input);

        assert_eq!(removed, 2);
        assert_eq!(
            output["tools"],
            json!([{"type": "function", "name": "generate_image_caption"}])
        );
    }
}
