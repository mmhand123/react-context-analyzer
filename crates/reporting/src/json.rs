use context_analyzer_core::model::{
    ComponentNode, FileFacts, ProjectFacts, ResolvedRenderEdge, UnresolvedRenderEdge,
};

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

    let mut normalized_graph = project_facts.graph.clone();
    normalize_graph(&mut normalized_graph);

    ProjectFacts {
        summary: project_facts.summary.clone(),
        files: normalized_files,
        graph: normalized_graph,
    }
}

fn normalize_graph(graph: &mut context_analyzer_core::model::ProjectGraph) {
    graph.components.sort_by(component_node_sort_key);

    graph
        .resolved_render_edges
        .sort_by(resolved_render_edge_sort_key);

    graph
        .unresolved_render_edges
        .sort_by(unresolved_render_edge_sort_key);
}

fn component_node_sort_key(left: &ComponentNode, right: &ComponentNode) -> std::cmp::Ordering {
    left.id
        .file_path
        .cmp(&right.id.file_path)
        .then_with(|| left.id.component_name.cmp(&right.id.component_name))
}

fn resolved_render_edge_sort_key(
    left: &ResolvedRenderEdge,
    right: &ResolvedRenderEdge,
) -> std::cmp::Ordering {
    left.parent_component_id
        .file_path
        .cmp(&right.parent_component_id.file_path)
        .then_with(|| {
            left.parent_component_id
                .component_name
                .cmp(&right.parent_component_id.component_name)
        })
        .then_with(|| {
            left.child_component_id
                .file_path
                .cmp(&right.child_component_id.file_path)
        })
        .then_with(|| {
            left.child_component_id
                .component_name
                .cmp(&right.child_component_id.component_name)
        })
}

fn unresolved_render_edge_sort_key(
    left: &UnresolvedRenderEdge,
    right: &UnresolvedRenderEdge,
) -> std::cmp::Ordering {
    left.parent_component_id
        .file_path
        .cmp(&right.parent_component_id.file_path)
        .then_with(|| {
            left.parent_component_id
                .component_name
                .cmp(&right.parent_component_id.component_name)
        })
        .then_with(|| left.child_symbol.cmp(&right.child_symbol))
        .then_with(|| left.reason.cmp(&right.reason))
}

fn normalize_file_facts(file_facts: &mut FileFacts) {
    file_facts
        .contexts
        .sort_by(|left_context, right_context| left_context.name.cmp(&right_context.name));

    file_facts
        .components
        .sort_by(|left_component, right_component| left_component.name.cmp(&right_component.name));

    file_facts
        .module_imports
        .sort_by(|left_import, right_import| {
            left_import
                .source_module
                .cmp(&right_import.source_module)
                .then_with(|| left_import.local_name.cmp(&right_import.local_name))
                .then_with(|| left_import.imported_name.cmp(&right_import.imported_name))
        });

    file_facts
        .module_exports
        .sort_by(|left_export, right_export| {
            left_export
                .export_name
                .cmp(&right_export.export_name)
                .then_with(|| left_export.local_name.cmp(&right_export.local_name))
                .then_with(|| left_export.source_module.cmp(&right_export.source_module))
        });

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
        ComponentDef, ComponentId, ComponentNode, ConsumerUse, ContextDef, ContextRef, ExportKind,
        ExportSymbol, FileFacts, FunctionOwnerKind, ImportKind, ImportSymbol, ProjectFacts,
        ProviderUse, RenderEdge, ResolvedRenderEdge, Span, UnresolvedRenderEdge,
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
        project_facts_variant_two.graph.components.reverse();
        project_facts_variant_two
            .graph
            .resolved_render_edges
            .reverse();
        project_facts_variant_two
            .graph
            .unresolved_render_edges
            .reverse();

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
        assert!(json_value.get("graph").is_some());
        assert!(json_value.get("diagnostics").is_none());
    }

    #[test]
    fn json_does_not_include_raw_module_import_or_export_lists() {
        let project_facts = build_project_facts(vec!["src/App.tsx"]);
        let json_output = to_json_pretty(&project_facts).expect("json should serialize");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_output).expect("json should parse");

        let files = json_value["files"]
            .as_array()
            .expect("files should be an array");
        let first_file = files.first().expect("fixture should contain one file");

        assert!(first_file.get("module_imports").is_none());
        assert!(first_file.get("module_exports").is_none());
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
                module_imports: vec![
                    ImportSymbol {
                        source_module: "./zebra".to_string(),
                        local_name: "ZebraPage".to_string(),
                        imported_name: Some("ZebraPage".to_string()),
                        kind: ImportKind::Named,
                        is_type_only: false,
                    },
                    ImportSymbol {
                        source_module: "./alpha".to_string(),
                        local_name: "DefaultAlpha".to_string(),
                        imported_name: Some("default".to_string()),
                        kind: ImportKind::Default,
                        is_type_only: false,
                    },
                ],
                module_exports: vec![
                    ExportSymbol {
                        export_name: "App".to_string(),
                        local_name: Some("App".to_string()),
                        source_module: None,
                        kind: ExportKind::Named,
                        is_type_only: false,
                    },
                    ExportSymbol {
                        export_name: "default".to_string(),
                        local_name: Some("App".to_string()),
                        source_module: None,
                        kind: ExportKind::Default,
                        is_type_only: false,
                    },
                ],
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

        let mut project_facts = ProjectFacts::from_files(files);
        project_facts.graph.components = vec![
            ComponentNode {
                id: ComponentId {
                    file_path: "src/Zebra.tsx".to_string(),
                    component_name: "ZebraPage".to_string(),
                },
            },
            ComponentNode {
                id: ComponentId {
                    file_path: "src/Alpha.tsx".to_string(),
                    component_name: "AlphaPage".to_string(),
                },
            },
        ];
        project_facts.graph.resolved_render_edges = vec![ResolvedRenderEdge {
            parent_component_id: ComponentId {
                file_path: "src/Zebra.tsx".to_string(),
                component_name: "ZebraPage".to_string(),
            },
            child_component_id: ComponentId {
                file_path: "src/Alpha.tsx".to_string(),
                component_name: "AlphaPage".to_string(),
            },
            span: Span::new(71, 80),
        }];
        project_facts.graph.unresolved_render_edges = vec![UnresolvedRenderEdge {
            parent_component_id: ComponentId {
                file_path: "src/Alpha.tsx".to_string(),
                component_name: "AlphaPage".to_string(),
            },
            child_symbol: "UnknownWidget".to_string(),
            span: Span::new(81, 90),
            reason: "symbol_not_found".to_string(),
        }];

        project_facts
    }
}
