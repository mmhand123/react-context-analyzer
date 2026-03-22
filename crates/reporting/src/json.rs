use context_analyzer_core::model::{FileInfo, ProjectInfo};

pub fn to_json_pretty(project_info: &ProjectInfo) -> Result<String, serde_json::Error> {
    let normalized_info = normalized_project_info(project_info);
    serde_json::to_string_pretty(&normalized_info)
}

pub fn to_json_compact(project_info: &ProjectInfo) -> Result<String, serde_json::Error> {
    let normalized_info = normalized_project_info(project_info);
    serde_json::to_string(&normalized_info)
}

fn normalized_project_info(project_info: &ProjectInfo) -> ProjectInfo {
    let mut normalized_files = project_info.files.clone();
    normalized_files
        .sort_by(|left_file, right_file| left_file.file_path.cmp(&right_file.file_path));

    for file_info in &mut normalized_files {
        normalize_file_info(file_info);
    }

    let normalized_graph = project_info.graph.clone();

    ProjectInfo {
        summary: project_info.summary.clone(),
        files: normalized_files,
        graph: normalized_graph,
    }
}

fn normalize_file_info(file_info: &mut FileInfo) {
    file_info
        .contexts
        .sort_by(|left_context, right_context| left_context.name.cmp(&right_context.name));

    file_info
        .components
        .sort_by(|left_component, right_component| left_component.name.cmp(&right_component.name));

    file_info
        .module_imports
        .sort_by(|left_import, right_import| {
            left_import
                .source_module
                .cmp(&right_import.source_module)
                .then_with(|| left_import.local_name.cmp(&right_import.local_name))
                .then_with(|| left_import.imported_name.cmp(&right_import.imported_name))
        });

    file_info
        .module_exports
        .sort_by(|left_export, right_export| {
            left_export
                .export_name
                .cmp(&right_export.export_name)
                .then_with(|| left_export.local_name.cmp(&right_export.local_name))
                .then_with(|| left_export.source_module.cmp(&right_export.source_module))
        });

    file_info
        .providers
        .sort_by(|left_provider, right_provider| {
            left_provider
                .context_ref
                .symbol
                .cmp(&right_provider.context_ref.symbol)
        });

    file_info
        .consumers
        .sort_by(|left_consumer, right_consumer| {
            left_consumer
                .context_ref
                .symbol
                .cmp(&right_consumer.context_ref.symbol)
        });

    file_info
        .unresolved_render_edges
        .sort_by(|left_edge, right_edge| {
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
        ComponentDef, ConsumerUse, ContextDef, ContextRef, ExportKind, ExportSymbol, FileInfo,
        FunctionOwnerKind, ImportKind, ImportSymbol, ProjectInfo, ProviderUse, ResolvedRenderEdge,
        Span, UnresolvedRenderEdge,
    };

    use super::{to_json_compact, to_json_pretty};

    #[test]
    fn json_does_not_include_diagnostics_field() {
        let project_info = ProjectInfo::from_files(vec![]);
        let json_output = to_json_pretty(&project_info).expect("json should serialize");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_output).expect("json should parse");

        assert!(json_value.get("summary").is_some());
        assert!(json_value.get("files").is_some());
        assert!(json_value.get("graph").is_some());
        assert!(json_value.get("diagnostics").is_none());
    }

    #[test]
    fn json_does_not_include_raw_module_import_or_export_lists() {
        let project_info = build_project_info(vec!["src/App.tsx"]);
        let json_output = to_json_pretty(&project_info).expect("json should serialize");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_output).expect("json should parse");

        let files = json_value["files"]
            .as_array()
            .expect("files should be an array");
        let first_file = files.first().expect("fixture should contain one file");

        assert!(first_file.get("module_imports").is_none());
        assert!(first_file.get("module_exports").is_none());
    }

    fn build_project_info(file_paths: Vec<&str>) -> ProjectInfo {
        let mut files = Vec::new();

        for file_path in file_paths {
            files.push(FileInfo {
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
                    key: "src/App.tsx:App".to_string(),
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
                unresolved_render_edges: vec![UnresolvedRenderEdge {
                    parent_component_name: "App".to_string(),
                    child_component_name: "ProfilePage".to_string(),
                    span: Span::new(61, 70),
                    parent_component_key: "src/App.tsx:App".to_string(),
                }],
            });
        }

        let mut project_info = ProjectInfo::from_files(files);
        // TODO: Probably fix this idk if it's actually useful
        // project_info.graph.components = vec![
        //     ComponentNode {
        //         id: ComponentId {
        //             file_path: "src/Zebra.tsx".to_string(),
        //             component_name: "ZebraPage".to_string(),
        //         },
        //     },
        //     ComponentNode {
        //         id: ComponentId {
        //             file_path: "src/Alpha.tsx".to_string(),
        //             component_name: "AlphaPage".to_string(),
        //         },
        //     },
        // ];
        // project_info.graph.resolved_render_edges = vec![ResolvedRenderEdge {
        //     parent_component_id: ComponentId {
        //         file_path: "src/Zebra.tsx".to_string(),
        //         component_name: "ZebraPage".to_string(),
        //     },
        //     child_component_id: ComponentId {
        //         file_path: "src/Alpha.tsx".to_string(),
        //         component_name: "AlphaPage".to_string(),
        //     },
        //     span: Span::new(71, 80),
        // }];
        //
        project_info
    }
}
