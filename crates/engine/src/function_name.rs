use context_analyzer_core::model::FunctionOwnerKind;
use oxc_ast::ast::{Expression, Function, JSXElementName, VariableDeclarator};

pub(crate) fn extract_declared_function_name(function_node: &Function<'_>) -> Option<String> {
    function_node
        .id
        .as_ref()
        .map(|identifier| identifier.name.as_str().to_string())
}

pub(crate) fn extract_function_name_from_variable_declarator(
    declarator: &VariableDeclarator<'_>,
) -> Option<String> {
    let binding_identifier = declarator.id.get_binding_identifier()?;
    let initializer_expression = declarator.init.as_ref()?;

    if matches!(
        initializer_expression,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    ) {
        return Some(binding_identifier.name.as_str().to_string());
    }

    None
}

pub(crate) fn extract_component_name_from_function(function_node: &Function<'_>) -> Option<String> {
    let function_name = extract_declared_function_name(function_node)?;
    if is_pascal_case(&function_name) {
        return Some(function_name);
    }

    None
}

pub(crate) fn extract_component_name_from_variable_declarator(
    declarator: &VariableDeclarator<'_>,
) -> Option<String> {
    let function_name = extract_function_name_from_variable_declarator(declarator)?;
    if is_pascal_case(&function_name) {
        return Some(function_name);
    }

    None
}

pub(crate) fn classify_function_owner_kind(function_name: &str) -> FunctionOwnerKind {
    if is_hook_name(function_name) {
        FunctionOwnerKind::Hook
    } else if is_pascal_case(function_name) {
        FunctionOwnerKind::Component
    } else {
        FunctionOwnerKind::Other
    }
}

pub(crate) fn is_pascal_case(name: &str) -> bool {
    name.chars()
        .next()
        .map(|first_character| first_character.is_ascii_uppercase())
        .unwrap_or(false)
}

fn is_hook_name(name: &str) -> bool {
    if !name.starts_with("use") {
        return false;
    }

    let potential_hook_name = name.get(3..).unwrap_or("");

    let first_char = potential_hook_name.chars().next();

    if first_char.is_none() || !first_char.unwrap().is_ascii_uppercase() {
        return false;
    }

    return true;
}

pub(crate) fn extract_jsx_component_name(element_name: &JSXElementName<'_>) -> Option<String> {
    match element_name {
        JSXElementName::Identifier(identifier) => {
            let candidate_name = identifier.name.as_str();
            if is_pascal_case(candidate_name) {
                Some(candidate_name.to_string())
            } else {
                None
            }
        }
        JSXElementName::IdentifierReference(identifier) => {
            let candidate_name = identifier.name.as_str();
            if is_pascal_case(candidate_name) {
                Some(candidate_name.to_string())
            } else {
                None
            }
        }
        JSXElementName::MemberExpression(member_expression) => {
            if let Some(full_name) = jsx_member_expression_full_name(member_expression)
                && is_pascal_case(full_name.split('.').next().unwrap_or_default())
            {
                return Some(full_name);
            }
            None
        }
        _ => None,
    }
}

fn jsx_member_expression_full_name(
    member_expression: &oxc_ast::ast::JSXMemberExpression<'_>,
) -> Option<String> {
    match &member_expression.object {
        oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => Some(format!(
            "{}.{}",
            identifier.name.as_str(),
            member_expression.property.name.as_str()
        )),
        oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(inner_member) => {
            let prefix = jsx_member_expression_full_name(inner_member)?;
            Some(format!(
                "{}.{}",
                prefix,
                member_expression.property.name.as_str()
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use context_analyzer_core::model::FunctionOwnerKind;
    use oxc_allocator::Allocator;
    use oxc_allocator::Box as ArenaBox;
    use oxc_ast::ast::{
        IdentifierReference, JSXElementName, JSXIdentifier, JSXMemberExpression,
        JSXMemberExpressionObject,
    };
    use oxc_span::Span;
    use oxc_syntax::node::NodeId;

    use super::{classify_function_owner_kind, extract_jsx_component_name, is_pascal_case};

    #[test]
    fn pascal_case_detection_is_readable_and_predictable() {
        assert!(is_pascal_case("App"));
        assert!(is_pascal_case("ProfilePage"));

        assert!(!is_pascal_case("div"));
        assert!(!is_pascal_case("profilePage"));
        assert!(!is_pascal_case(""));
    }

    #[test]
    fn function_owner_classification_distinguishes_component_hook_and_other() {
        assert_eq!(
            classify_function_owner_kind("App"),
            FunctionOwnerKind::Component
        );
        assert_eq!(
            classify_function_owner_kind("useAuth"),
            FunctionOwnerKind::Hook
        );
        assert_eq!(
            classify_function_owner_kind("helper"),
            FunctionOwnerKind::Other
        );
    }

    #[test]
    fn jsx_tag_component_name_extraction_handles_identifier_and_member_cases() {
        let allocator = Allocator::default();

        let identifier_reference_name = jsx_identifier_reference_name(&allocator, "ProfilePage");
        assert_eq!(
            extract_jsx_component_name(&identifier_reference_name),
            Some("ProfilePage".to_string())
        );

        let member_name = jsx_member_name(&allocator, &["UI", "Button"]);
        assert_eq!(
            extract_jsx_component_name(&member_name),
            Some("UI.Button".to_string())
        );

        let deep_member_name = jsx_member_name(&allocator, &["UI", "Nav", "Item"]);
        assert_eq!(
            extract_jsx_component_name(&deep_member_name),
            Some("UI.Nav.Item".to_string())
        );

        let intrinsic_identifier_name = jsx_identifier_name(&allocator, "div");
        assert_eq!(extract_jsx_component_name(&intrinsic_identifier_name), None);

        let lowercase_member_name = jsx_member_name(&allocator, &["ui", "Button"]);
        assert_eq!(extract_jsx_component_name(&lowercase_member_name), None);
    }

    fn jsx_identifier_name<'a>(allocator: &'a Allocator, name: &'a str) -> JSXElementName<'a> {
        JSXElementName::Identifier(ArenaBox::new_in(
            JSXIdentifier {
                node_id: Cell::new(NodeId::DUMMY),
                span: Span::new(0, 0),
                name: name.into(),
            },
            allocator,
        ))
    }

    fn jsx_identifier_reference_name<'a>(
        allocator: &'a Allocator,
        name: &'a str,
    ) -> JSXElementName<'a> {
        JSXElementName::IdentifierReference(ArenaBox::new_in(
            IdentifierReference {
                node_id: Cell::new(NodeId::DUMMY),
                span: Span::new(0, 0),
                name: name.into(),
                reference_id: Cell::new(None),
            },
            allocator,
        ))
    }

    fn jsx_member_name<'a>(allocator: &'a Allocator, segments: &[&'a str]) -> JSXElementName<'a> {
        assert!(
            segments.len() >= 2,
            "member expression needs at least two segments"
        );

        let mut object = JSXMemberExpressionObject::IdentifierReference(ArenaBox::new_in(
            IdentifierReference {
                node_id: Cell::new(NodeId::DUMMY),
                span: Span::new(0, 0),
                name: segments[0].into(),
                reference_id: Cell::new(None),
            },
            allocator,
        ));

        for segment in &segments[1..segments.len() - 1] {
            let nested_member = JSXMemberExpression {
                node_id: Cell::new(NodeId::DUMMY),
                span: Span::new(0, 0),
                object,
                property: JSXIdentifier {
                    node_id: Cell::new(NodeId::DUMMY),
                    span: Span::new(0, 0),
                    name: (*segment).into(),
                },
            };
            object = JSXMemberExpressionObject::MemberExpression(ArenaBox::new_in(
                nested_member,
                allocator,
            ));
        }

        let property_name = segments[segments.len() - 1];
        JSXElementName::MemberExpression(ArenaBox::new_in(
            JSXMemberExpression {
                node_id: Cell::new(NodeId::DUMMY),
                span: Span::new(0, 0),
                object,
                property: JSXIdentifier {
                    node_id: Cell::new(NodeId::DUMMY),
                    span: Span::new(0, 0),
                    name: property_name.into(),
                },
            },
            allocator,
        ))
    }
}
