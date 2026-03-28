use std::collections::HashMap;
use std::path::Path;

use context_analyzer_core::model::{
    Component, ComponentKey, ExportKind, ExportSymbol, ExportSymbolKey, FileInfo, ImportKind,
    ProjectGraph, ResolvedRenderEdge,
};
use oxc_resolver::{ResolveOptions, Resolver, TsconfigDiscovery};
use rayon::prelude::*;

use crate::paths::{normalize_file_path_from_path, normalize_file_path_string};

/// For our graph we're mainly taking the render edges we've gotten walking the ASTs for all of the
/// files, and then resolving imports back to the actual file the rendered component comes from.
/// We'll use this to be able to walk up the graph and ensure each component that uses a context
/// has the appropriate Provider rendered above it
pub fn build_project_graph(files: &[FileInfo]) -> ProjectGraph {
    let resolver = Resolver::new(resolve_options());

    let mut graph = ProjectGraph::default();

    // TODO: We might want to move this to where we're loading the files, we shouldn't have to care about
    // this in here
    let mut files_in_stable_order: Vec<&FileInfo> = files.iter().collect();
    files_in_stable_order.sort_by(|left, right| {
        normalize_file_path_string(&left.file_path)
            .cmp(&normalize_file_path_string(&right.file_path))
    });

    // We need to ensure node_id is correct across all files, but want to do the actual work in
    // parallel. Might be another way to do this I don't love it
    let mut next_component_node_id = 0;
    let mut file_component_offsets = Vec::with_capacity(files_in_stable_order.len());
    for file_info in &files_in_stable_order {
        file_component_offsets.push(next_component_node_id);
        next_component_node_id += file_info.components.len();
    }

    let (components_map, exports_map): (
        HashMap<ComponentKey, Component>,
        HashMap<ExportSymbolKey, ExportSymbol>,
    ) = files_in_stable_order
        .par_iter()
        .zip(file_component_offsets.par_iter())
        .fold(
            || (HashMap::new(), HashMap::new()),
            |(mut components, mut exports), (file_info, component_offset)| {
                let normalized_file_path = normalize_file_path_string(&file_info.file_path);

                for (component_idx, component_def) in file_info.components.iter().enumerate() {
                    let component = Component::new(
                        &normalized_file_path,
                        &component_def.name,
                        component_offset + component_idx,
                        component_def.span,
                    );

                    components.insert(
                        (normalized_file_path.clone(), component_def.name.clone()),
                        component,
                    );
                }

                for export_symbol in &file_info.module_exports {
                    let export_name = match export_symbol.kind {
                        ExportKind::Named => export_symbol.export_name.clone(),
                        ExportKind::Default => "default".to_string(),
                        _ => export_symbol.local_name.clone().unwrap_or_default(),
                    };

                    exports.insert(
                        (normalized_file_path.clone(), export_name),
                        export_symbol.clone(),
                    );
                }

                (components, exports)
            },
        )
        .reduce(
            || (HashMap::new(), HashMap::new()),
            |(mut c1, mut e1), (c2, e2)| {
                c1.extend(c2);
                e1.extend(e2);
                (c1, e1)
            },
        );

    graph.resolved_render_edges = vec![Vec::new(); next_component_node_id];

    // We'd like to do this in parallel but we're going to have to work around sharing the hashmap
    for file_info in &files_in_stable_order {
        for edge in &file_info.unresolved_render_edges {
            let current_file_path = normalize_file_path_string(&file_info.file_path);
            // TODO - figure out all this clone nonsense
            let parent_component = components_map.get(&(
                current_file_path.clone(),
                edge.parent_component_name.clone(),
            ));

            if let Some(parent_component) = parent_component
                && let Some(child_component) = resolve_child_component(
                    file_info,
                    &edge.child_rendered_symbol,
                    &resolver,
                    &current_file_path,
                    &components_map,
                    &exports_map,
                )
            {
                if edge.parent_jsx_symbol == edge.parent_component_name {
                    graph.resolved_render_edges[parent_component.node_id].push(
                        ResolvedRenderEdge {
                            parent_component_id: parent_component.node_id,
                            child_component_id: child_component.node_id,
                            parent_jsx_component_id: parent_component.node_id,
                            span: edge.span,
                        },
                    );
                } else if let Some(parent_jsx_component) = resolve_child_component(
                    file_info,
                    &edge.parent_jsx_symbol,
                    &resolver,
                    &current_file_path,
                    &components_map,
                    &exports_map,
                ) {
                    graph.resolved_render_edges[parent_component.node_id].push(
                        ResolvedRenderEdge {
                            parent_component_id: parent_component.node_id,
                            child_component_id: child_component.node_id,
                            parent_jsx_component_id: parent_jsx_component.node_id,
                            span: edge.span,
                        },
                    );
                }
            }
        }
    }

    graph.components = vec![Component::default(); next_component_node_id];
    for component in components_map.values() {
        graph.components[component.node_id] = component.clone();
    }
    graph.components_by_key = components_map.clone();

    graph
}

fn resolve_child_component(
    file_info: &FileInfo,
    child_symbol: &str,
    resolver: &Resolver,
    current_file_path: &String,
    components_map: &HashMap<ComponentKey, Component>,
    exports_map: &HashMap<ExportSymbolKey, ExportSymbol>,
) -> Option<Component> {
    // TODO: I think if child_symbol is "children" we need to return a Component with a predefined
    // node_id, something like -1 that we know is always children. Means we need to move away from
    // usize for node_id thoough...
    // Or we can resolve children separately somehow
    let import_symbol = file_info.module_imports.iter().find(|import_symbol| {
        if import_symbol.is_type_only {
            return false;
        }

        match import_symbol.kind {
            ImportKind::Named => import_symbol.local_name == child_symbol,
            ImportKind::Namespace => child_symbol.starts_with(&import_symbol.local_name),
            ImportKind::Default => import_symbol.local_name == child_symbol,
        }
    })?;

    let resolve_result = resolver
        .resolve_file(Path::new(&current_file_path), &import_symbol.source_module)
        .ok()?;

    let resolved_file_path = normalize_file_path_from_path(resolve_result.path());

    let export_symbol =
        exports_map.get(&(resolved_file_path.clone(), import_symbol.local_name.clone()));

    // Handle named exports/export aliases explicitly
    if let Some(export_symbol) = export_symbol {
        let export_local_component_name = export_symbol.local_name.clone().unwrap_or_default();

        return components_map
            .get(&(resolved_file_path, export_local_component_name))
            .cloned();
    }

    match import_symbol.kind {
        ImportKind::Default => {
            let export_default_symbol =
                exports_map.get(&(resolved_file_path.clone(), "default".to_string()));

            if let Some(export_default_symbol) = export_default_symbol {
                let export_default_component_name =
                    export_default_symbol.local_name.clone().unwrap_or_default();

                return components_map
                    .get(&(resolved_file_path, export_default_component_name))
                    .cloned();
            }

            // This would technically be an error but we're not here to parse syntax
            None
        }
        ImportKind::Named => {
            let imported_name = import_symbol
                .imported_name
                .as_deref()
                .unwrap_or(&import_symbol.local_name);
            components_map
                .get(&(resolved_file_path, imported_name.to_string()))
                .cloned()
        }
        ImportKind::Namespace => {
            let local_name = child_symbol
                .strip_prefix(&import_symbol.local_name)
                .and_then(|rest| rest.strip_prefix('.').or(Some(rest)))
                .unwrap_or(child_symbol);

            components_map
                .get(&(resolved_file_path, local_name.to_string()))
                .cloned()
        }
    }
}

fn resolve_options() -> ResolveOptions {
    ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        condition_names: vec!["node".to_string(), "import".to_string()],
        extensions: vec![
            ".tsx".to_string(),
            ".ts".to_string(),
            ".jsx".to_string(),
            ".js".to_string(),
            ".mjs".to_string(),
            ".cjs".to_string(),
        ],
        ..ResolveOptions::default()
    }
}
