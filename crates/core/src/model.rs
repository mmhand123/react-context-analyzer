use serde::Serialize;

pub use oxc_span::Span;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectFacts {
    pub summary: SummaryCounts,
    pub files: Vec<FileFacts>,
    pub graph: ProjectGraph,
}

impl ProjectFacts {
    pub fn from_files(files: Vec<FileFacts>) -> Self {
        let summary = SummaryCounts::from_files(&files);
        Self {
            summary,
            files,
            graph: ProjectGraph::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProjectGraph {
    pub components: Vec<ComponentNode>,
    pub resolved_render_edges: Vec<ResolvedRenderEdge>,
    pub unresolved_render_edges: Vec<UnresolvedRenderEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentId {
    pub file_path: String,
    pub component_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentNode {
    pub id: ComponentId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedRenderEdge {
    pub parent_component_id: ComponentId,
    pub child_component_id: ComponentId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnresolvedRenderEdge {
    pub parent_component_id: ComponentId,
    pub child_symbol: String,
    pub span: Span,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SummaryCounts {
    pub file_count: usize,
    pub context_count: usize,
    pub component_count: usize,
    pub provider_count: usize,
    pub consumer_count: usize,
    pub render_edge_count: usize,
}

impl SummaryCounts {
    pub fn from_files(files: &[FileFacts]) -> Self {
        let file_count = files.len();
        let mut context_count = 0;
        let mut component_count = 0;
        let mut provider_count = 0;
        let mut consumer_count = 0;
        let mut render_edge_count = 0;

        for file_facts in files {
            context_count += file_facts.contexts.len();
            component_count += file_facts.components.len();
            provider_count += file_facts.providers.len();
            consumer_count += file_facts.consumers.len();
            render_edge_count += file_facts.render_edges.len();
        }

        Self {
            file_count,
            context_count,
            component_count,
            provider_count,
            consumer_count,
            render_edge_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileFacts {
    pub file_path: String,
    pub contexts: Vec<ContextDef>,
    pub components: Vec<ComponentDef>,
    pub providers: Vec<ProviderUse>,
    pub consumers: Vec<ConsumerUse>,
    pub render_edges: Vec<RenderEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextDef {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentDef {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextRef {
    pub symbol: String,
    pub resolved_context_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionOwnerKind {
    Component,
    Hook,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderUse {
    pub context_ref: ContextRef,
    pub containing_component_name: Option<String>,
    pub containing_function_name: Option<String>,
    pub containing_function_kind: Option<FunctionOwnerKind>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsumerUse {
    pub context_ref: ContextRef,
    pub containing_component_name: Option<String>,
    pub containing_function_name: Option<String>,
    pub containing_function_kind: Option<FunctionOwnerKind>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderEdge {
    pub parent_component_name: String,
    pub child_component_name: String,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::{
        ComponentDef, ComponentId, ComponentNode, ConsumerUse, ContextDef, ContextRef, FileFacts,
        FunctionOwnerKind, ProjectFacts, ProviderUse, RenderEdge, ResolvedRenderEdge, Span,
        UnresolvedRenderEdge,
    };

    #[test]
    fn project_summary_counts_match_file_fact_totals() {
        let source_file_facts = vec![FileFacts {
            file_path: "src/App.tsx".to_string(),
            contexts: vec![ContextDef {
                name: "AuthContext".to_string(),
                span: Span::new(0, 10),
            }],
            components: vec![ComponentDef {
                name: "App".to_string(),
                span: Span::new(11, 30),
            }],
            providers: vec![ProviderUse {
                context_ref: ContextRef {
                    symbol: "AuthContext".to_string(),
                    resolved_context_id: None,
                },
                containing_component_name: Some("App".to_string()),
                containing_function_name: Some("App".to_string()),
                containing_function_kind: Some(FunctionOwnerKind::Component),
                span: Span::new(31, 50),
            }],
            consumers: vec![ConsumerUse {
                context_ref: ContextRef {
                    symbol: "AuthContext".to_string(),
                    resolved_context_id: None,
                },
                containing_component_name: Some("App".to_string()),
                containing_function_name: Some("App".to_string()),
                containing_function_kind: Some(FunctionOwnerKind::Component),
                span: Span::new(51, 70),
            }],
            render_edges: vec![RenderEdge {
                parent_component_name: "App".to_string(),
                child_component_name: "ProfilePage".to_string(),
                span: Span::new(71, 95),
            }],
        }];

        let project_facts = ProjectFacts::from_files(source_file_facts);

        assert_eq!(project_facts.summary.file_count, 1);
        assert_eq!(project_facts.summary.context_count, 1);
        assert_eq!(project_facts.summary.component_count, 1);
        assert_eq!(project_facts.summary.provider_count, 1);
        assert_eq!(project_facts.summary.consumer_count, 1);
        assert_eq!(project_facts.summary.render_edge_count, 1);
    }

    #[test]
    fn project_facts_serializes_without_diagnostics_field() {
        let project_facts = ProjectFacts::from_files(vec![]);
        let json_value =
            serde_json::to_value(project_facts).expect("project facts should serialize");

        assert!(json_value.get("summary").is_some());
        assert!(json_value.get("files").is_some());
        assert!(json_value.get("graph").is_some());
        assert!(json_value.get("diagnostics").is_none());
    }

    #[test]
    fn project_facts_graph_field_supports_component_and_edge_records() {
        let mut project_facts = ProjectFacts::from_files(vec![]);
        let app_component_id = ComponentId {
            file_path: "src/App.tsx".to_string(),
            component_name: "App".to_string(),
        };
        let profile_component_id = ComponentId {
            file_path: "src/ProfilePage.tsx".to_string(),
            component_name: "ProfilePage".to_string(),
        };

        project_facts.graph.components = vec![
            ComponentNode {
                id: app_component_id.clone(),
            },
            ComponentNode {
                id: profile_component_id.clone(),
            },
        ];
        project_facts.graph.resolved_render_edges = vec![ResolvedRenderEdge {
            parent_component_id: app_component_id.clone(),
            child_component_id: profile_component_id,
            span: Span::new(100, 120),
        }];
        project_facts.graph.unresolved_render_edges = vec![UnresolvedRenderEdge {
            parent_component_id: app_component_id,
            child_symbol: "LazyWidget".to_string(),
            span: Span::new(121, 140),
            reason: "symbol_not_found".to_string(),
        }];

        let json_value =
            serde_json::to_value(project_facts).expect("project facts should serialize");

        assert_eq!(
            json_value["graph"]["components"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(
            json_value["graph"]["resolved_render_edges"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            json_value["graph"]["unresolved_render_edges"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }
}
