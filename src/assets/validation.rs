//! JSON schema validation for character definitions and rig files.
//!
//! Validates that character JSON files contain valid fields with correct
//! ranges and enum values before they are loaded.

use crate::errors::AnimError;
use crate::procedural::CharacterDesc;

/// Validate a character description JSON.
///
/// Checks:
/// - All required fields are present
/// - Numeric fields are within valid ranges
/// - Enum values are valid
/// - Color values are in range 0-255
pub fn validate_character(desc: &CharacterDesc) -> Result<(), AnimError> {
    // Validate body fields.
    validate_range("body.height", desc.body.height, 0.0, 1.0)?;
    validate_range("body.build", desc.body.build, 0.0, 1.0)?;
    validate_color("body.skin_color", &desc.body.skin_color)?;

    // Validate face fields.
    validate_range("face.shape", desc.face.shape, 0.0, 1.0)?;
    validate_range("face.eye_size", desc.face.eye_size, 0.5, 1.2)?;
    validate_color("face.eye_color", &desc.face.eye_color)?;
    validate_range(
        "face.eyebrow_thickness",
        desc.face.eyebrow_thickness,
        0.0,
        1.0,
    )?;
    validate_range("face.nose_size", desc.face.nose_size, 0.0, 1.0)?;
    validate_range("face.lip_fullness", desc.face.lip_fullness, 0.0, 1.0)?;

    // Validate hair fields.
    validate_color("hair.color", &desc.hair.color)?;
    validate_range("hair.length", desc.hair.length, 0.0, 1.0)?;
    // HairStyle is validated by deserialization, but we check it's not invalid.
    // Serde will fail if the enum variant is unknown.

    // Validate outfit fields.
    // Top: kind is validated by deserialization.
    validate_color("outfit.top.color", &desc.outfit.top.color)?;
    if let Some(sec) = &desc.outfit.top.secondary_color {
        validate_color("outfit.top.secondary_color", sec)?;
    }

    // Bottom.
    validate_color("outfit.bottom.color", &desc.outfit.bottom.color)?;

    // Shoes.
    validate_color("outfit.shoes.color", &desc.outfit.shoes.color)?;

    // Accessories.
    for (i, acc) in desc.outfit.accessories.iter().enumerate() {
        let prefix = format!("outfit.accessories[{}]", i);
        validate_color(&format!("{}.color", prefix), &acc.color)?;
    }

    Ok(())
}

/// Validate that a value is within the expected range.
fn validate_range(field: &str, value: f64, min: f64, max: f64) -> Result<(), AnimError> {
    if value < min || value > max {
        return Err(AnimError::Asset(format!(
            "Field '{}' has value {} which is outside the valid range [{}, {}]",
            field, value, min, max
        )));
    }
    Ok(())
}

/// Validate that a color value is within 0-255.
fn validate_color(field: &str, color: &[u8; 3]) -> Result<(), AnimError> {
    for (i, &c) in color.iter().enumerate() {
        if c > 255 {
            return Err(AnimError::Asset(format!(
                "Color field '{}' has component {} with value {} which exceeds 255",
                field, i, c
            )));
        }
    }
    Ok(())
}

/// Validate a complete character JSON string.
pub fn validate_character_json(json: &str) -> Result<CharacterDesc, AnimError> {
    let desc: CharacterDesc = serde_json::from_str(json)
        .map_err(|e| AnimError::Asset(format!("Failed to parse character JSON: {}", e)))?;

    validate_character(&desc)?;
    Ok(desc)
}

/// Validate a rig JSON definition.
pub fn validate_rig_json(json: &str) -> Result<(), AnimError> {
    // Parse as serde_json::Value to validate structure.
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| AnimError::Asset(format!("Failed to parse rig JSON: {}", e)))?;

    // Check required fields.
    let required_fields = ["name", "height", "skeleton", "poses"];
    for field in &required_fields {
        if !v.get(*field).is_some() {
            return Err(AnimError::Asset(format!(
                "Rig JSON missing required field: '{}'",
                field
            )));
        }
    }

    // Validate height is a number.
    if let Some(height) = v.get("height").and_then(|h| h.as_f64()) {
        if height <= 0.0 || height > 500.0 {
            return Err(AnimError::Asset(format!(
                "Rig height {} is outside valid range (0, 500]",
                height
            )));
        }
    }

    // Validate skeleton has at least a root bone.
    if let Some(skeleton) = v.get("skeleton") {
        if let Some(root) = skeleton.get("root") {
            if root.get("name").is_none() {
                return Err(AnimError::Asset(
                    "Skeleton root bone must have a 'name' field".into(),
                ));
            }
        } else {
            return Err(AnimError::Asset("Skeleton must have a 'root' bone".into()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_character() {
        let json = r#"
        {
            "name": "test",
            "body": {
                "height": 0.5,
                "build": 0.5,
                "skin_color": [200, 180, 160]
            },
            "face": {
                "shape": 0.5,
                "eye_size": 1.0,
                "eye_color": [80, 100, 120],
                "eyebrow_thickness": 0.5,
                "nose_size": 0.5,
                "lip_fullness": 0.5
            },
            "hair": {
                "color": [60, 40, 30],
                "style": "Straight",
                "length": 0.5
            },
            "outfit": {
                "top": {
                    "kind": "TShirt",
                    "color": [100, 120, 140]
                },
                "bottom": {
                    "kind": "Pants",
                    "color": [60, 60, 70]
                },
                "shoes": {
                    "kind": "Sneakers",
                    "color": [40, 40, 40]
                },
                "accessories": []
            }
        }
        "#;

        let result = validate_character_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_height() {
        let json = r#"
        {
            "name": "test",
            "body": {
                "height": 2.5,
                "build": 0.5,
                "skin_color": [200, 180, 160]
            },
            "face": {
                "shape": 0.5,
                "eye_size": 1.0,
                "eye_color": [80, 100, 120],
                "eyebrow_thickness": 0.5,
                "nose_size": 0.5,
                "lip_fullness": 0.5
            },
            "hair": {
                "color": [60, 40, 30],
                "style": "Straight",
                "length": 0.5
            },
            "outfit": {
                "top": {
                    "kind": "TShirt",
                    "color": [100, 120, 140]
                },
                "bottom": {
                    "kind": "Pants",
                    "color": [60, 60, 70]
                },
                "shoes": {
                    "kind": "Sneakers",
                    "color": [40, 40, 40]
                },
                "accessories": []
            }
        }
        "#;

        let result = validate_character_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("body.height"));
    }

    #[test]
    fn test_invalid_color() {
        let json = r#"
        {
            "name": "test",
            "body": {
                "height": 0.5,
                "build": 0.5,
                "skin_color": [200, 180, 300]
            },
            "face": {
                "shape": 0.5,
                "eye_size": 1.0,
                "eye_color": [80, 100, 120],
                "eyebrow_thickness": 0.5,
                "nose_size": 0.5,
                "lip_fullness": 0.5
            },
            "hair": {
                "color": [60, 40, 30],
                "style": "Straight",
                "length": 0.5
            },
            "outfit": {
                "top": {
                    "kind": "TShirt",
                    "color": [100, 120, 140]
                },
                "bottom": {
                    "kind": "Pants",
                    "color": [60, 60, 70]
                },
                "shoes": {
                    "kind": "Sneakers",
                    "color": [40, 40, 40]
                },
                "accessories": []
            }
        }
        "#;

        let result = validate_character_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("skin_color"));
    }

    #[test]
    fn test_valid_rig() {
        let json = r#"
        {
            "name": "test_rig",
            "height": 180.0,
            "skeleton": {
                "root": {
                    "name": "torso",
                    "part": "torso",
                    "pivot": [0.5, 0.5],
                    "offset": [0.0, 0.0],
                    "rotation": 0.0,
                    "scale": [1.0, 1.0],
                    "z_order": 0,
                    "children": [
                        {
                            "name": "head",
                            "part": "head",
                            "pivot": [0.5, 0.0],
                            "offset": [0.0, -50.0],
                            "rotation": 0.0,
                            "scale": [1.0, 1.0],
                            "z_order": 1,
                            "children": []
                        }
                    ]
                }
            },
            "poses": {
                "idle": {
                    "name": "idle",
                    "bones": {},
                    "transition_duration": 0.3
                }
            }
        }
        "#;

        let result = validate_rig_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_rig_missing_root() {
        let json = r#"
        {
            "name": "test_rig",
            "height": 180.0,
            "skeleton": {},
            "poses": {}
        }
        "#;

        let result = validate_rig_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("root"));
    }
}
