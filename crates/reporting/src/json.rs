use context_analyzer_core::model::{FileFacts, ProjectFacts};

pub fn to_json_pretty(project_facts: &ProjectFacts) -> Result<String, serde_json::Error> {
    let normalized_facts = normalized_project_facts(project_facts);
    serde_json::to_string_pretty(&normalized_facts)
}

pub fn to_json_compact(project_facts: &ProjectFacts) -> Result<String, serde_json::Error> {
    let normalized_facts = normalized_project_facts(project_facts);
    serde_json::to_string(&normalized_facts)
}

fn normalized_project_facts(project_facts: &ProjectFacts) -> ProjectFacts {
    let mut normalized_files = project_facts.files.clone();
    normalized_files
        .sort_by(|left_file, right_file| left_file.file_path.cmp(&right_file.file_path));

    for file_facts in &mut normalized_files {
        normalize_file_facts(file_facts);
    }

    ProjectFacts {
        summary: project_facts.summary.clone(),
        files: normalized_files,
    }
}

fn normalize_file_facts(file_facts: &mut FileFacts) {
    file_facts
        .contexts
        .sort_by(|left_context, right_context| left_context.name.cmp(&right_context.name));

    file_facts
        .components
        .sort_by(|left_component, right_component| left_component.name.cmp(&right_component.name));

    file_facts
        .providers
        .sort_by(|left_provider, right_provider| {
            left_provider
                .context_ref
                .symbol
                .cmp(&right_provider.context_ref.symbol)
        });

    file_facts
        .consumers
        .sort_by(|left_consumer, right_consumer| {
            left_consumer
                .context_ref
                .symbol
                .cmp(&right_consumer.context_ref.symbol)
        });

    file_facts.render_edges.sort_by(|left_edge, right_edge| {
        left_edge
            .parent_component_name
            .cmp(&right_edge.parent_component_name)
            .then_with(|| {
                left_edge
                    .child_component_name
                    .cmp(&right_edge.child_component_name)
            })
    });
}

#[cfg(test)]
mod tests {
    use context_analyzer_core::model::{
        ComponentDef, ConsumerUse, ContextDef, ContextRef, FileFacts, FunctionOwnerKind,
        ProjectFacts, ProviderUse, RenderEdge, Span,
    };

    use super::{to_json_compact, to_json_pretty};

    #[test]
    fn json_output_is_deterministic_when_input_order_varies() {
        let project_facts_variant_one = build_project_facts(vec!["src/Alpha.tsx", "src/Zebra.tsx"]);
        let mut project_facts_variant_two = project_facts_variant_one.clone();

        project_facts_variant_two.files.reverse();
        for file_facts in &mut project_facts_variant_two.files {
            file_facts.contexts.reverse();
            file_facts.components.reverse();
            file_facts.providers.reverse();
            file_facts.consumers.reverse();
            file_facts.render_edges.reverse();
        }

        let output_one =
            to_json_compact(&project_facts_variant_one).expect("json should serialize");
        let output_two =
            to_json_compact(&project_facts_variant_two).expect("json should serialize");

        assert_eq!(output_one, output_two);
    }

    #[test]
    fn json_does_not_include_diagnostics_field() {
        let project_facts = ProjectFacts::from_files(vec![]);
        let json_output = to_json_pretty(&project_facts).expect("json should serialize");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_output).expect("json should parse");

        assert!(json_value.get("summary").is_some());
        assert!(json_value.get("files").is_some());
        assert!(json_value.get("diagnostics").is_none());
    }

    fn build_project_facts(file_paths: Vec<&str>) -> ProjectFacts {
        let mut files = Vec::new();

        for file_path in file_paths {
            files.push(FileFacts {
                file_path: file_path.to_string(),
                contexts: vec![
                    ContextDef {
                        name: "AuthContext".to_string(),
                        span: Span::new(10, 20),
                    },
                    ContextDef {
                        name: "ThemeContext".to_string(),
                        span: Span::new(21, 30),
                    },
                ],
                components: vec![ComponentDef {
                    name: "App".to_string(),
                    span: Span::new(31, 40),
                }],
                providers: vec![ProviderUse {
                    context_ref: ContextRef {
                        symbol: "ThemeContext".to_string(),
                        resolved_context_id: None,
                    },
                    containing_component_name: Some("App".to_string()),
                    containing_function_name: Some("App".to_string()),
                    containing_function_kind: Some(FunctionOwnerKind::Component),
                    span: Span::new(41, 50),
                }],
                consumers: vec![ConsumerUse {
                    context_ref: ContextRef {
                        symbol: "AuthContext".to_string(),
                        resolved_context_id: None,
                    },
                    containing_component_name: Some("App".to_string()),
                    containing_function_name: Some("App".to_string()),
                    containing_function_kind: Some(FunctionOwnerKind::Component),
                    span: Span::new(51, 60),
                }],
                render_edges: vec![RenderEdge {
                    parent_component_name: "App".to_string(),
                    child_component_name: "ProfilePage".to_string(),
                    span: Span::new(61, 70),
                }],
            });
        }

        ProjectFacts::from_files(files)
    }
}
