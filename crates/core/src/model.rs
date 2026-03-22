use std::collections::HashMap;

use serde::Serialize;

pub use oxc_span::Span;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectInfo {
    pub summary: SummaryCounts,
    pub files: Vec<FileInfo>,
    pub graph: ProjectGraph,
}

impl ProjectInfo {
    pub fn from_files(files: Vec<FileInfo>) -> Self {
        let summary = SummaryCounts::from_files(&files);
        Self {
            summary,
            files,
            graph: ProjectGraph::default(),
        }
    }
}

pub type ComponentName = String;
pub type FilePath = String;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProjectGraph {
    pub components: Vec<Component>,
    pub components_by_key: HashMap<ComponentKey, Component>,
    /// When we resolve the edge, we'll store the resolved child component as the key and then
    /// the full edge as the value. This lets us walk the graph in reverse order.
    pub resolved_render_edges: HashMap<ComponentKey, ResolvedRenderEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedRenderEdge {
    pub parent_component_id: usize,
    pub child_component_id: usize,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Component {
    pub node_id: usize,
    pub key: ComponentKey,
    pub file_path: String,
    pub name: String,
}

impl Component {
    pub fn new(file_path: &str, component_name: &str) -> Self {
        Self {
            key: get_component_key(file_path, component_name),
            file_path: file_path.to_string(),
            name: component_name.to_string(),
        }
    }
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
    pub fn from_files(files: &[FileInfo]) -> Self {
        let file_count = files.len();
        let mut context_count = 0;
        let mut component_count = 0;
        let mut provider_count = 0;
        let mut consumer_count = 0;
        let mut render_edge_count = 0;

        for file_info in files {
            context_count += file_info.contexts.len();
            component_count += file_info.components.len();
            provider_count += file_info.providers.len();
            consumer_count += file_info.consumers.len();
            render_edge_count += file_info.render_edges.len();
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
pub struct FileInfo {
    pub file_path: String,
    pub contexts: Vec<ContextDef>,
    pub components: Vec<ComponentDef>,
    #[serde(skip_serializing)]
    pub module_imports: Vec<ImportSymbol>,
    #[serde(skip_serializing)]
    pub module_exports: Vec<ExportSymbol>,
    pub providers: Vec<ProviderUse>,
    pub consumers: Vec<ConsumerUse>,
    pub render_edges: Vec<RenderEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    Named,
    Default,
    Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportSymbol {
    pub source_module: String,
    pub local_name: String,
    pub imported_name: Option<String>,
    pub kind: ImportKind,
    pub is_type_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportKind {
    Named,
    Default,
    ReExportAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportSymbol {
    pub export_name: String,
    pub local_name: Option<String>,
    pub source_module: Option<String>,
    pub kind: ExportKind,
    pub is_type_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextDef {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentDef {
    // TODO this needs an int id - we do want to store
    pub name: String,
    pub key: ComponentKey,
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

/// A unique identifier for a component - component name + file path
pub type ComponentKey = (FilePath, ComponentName);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderEdge {
    pub parent_node_id: usize,
    /// We only know the child component name on first pass because we have to resolve imports to
    /// get the node id
    pub child_rendered_symbol: String,
    pub span: Span,
}

pub fn get_component_key(file_path: &str, component_name: &str) -> ComponentKey {
    (file_path.to_string(), component_name.to_string())
}

// TODO - We need to keep track of the JSX render tree in each file as well, so that we can resolve
// that to figure out the provider stuff. We're basically going to need to resolve the whole render
// tree
// It should probably look like, for each component that renders other components - do a
// Vec<Vec<TreeNode>> where the tree node id is an auto incrementing usize
// hmm but we need to account for the nested relationship, will need to think about that
// it could be another adjacency list or could just be a raw list of nodes that point to their
// parent
