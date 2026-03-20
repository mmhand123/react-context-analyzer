use std::collections::HashMap;
use std::path::Path;

use context_analyzer_core::model::{
    Component, ComponentKey, ExportKind, FileInfo, ImportKind, ProjectGraph, ResolvedRenderEdge,
};
use oxc_resolver::{ResolveOptions, Resolver, TsconfigDiscovery};
use rayon::prelude::*;

use crate::paths::{normalize_file_path_from_path, normalize_file_path_string};

/// For our graph we're mainly taking the render edges we've gotten walking the ASTs for all of the
/// files, and then resolving imports back to the actual file the rendered component comes from.
/// We'll use this to be able to walk up the graph and ensure each component that uses a context
/// has the appropriate Provider rendered above it
pub fn build_project_graph(files: &Vec<FileInfo>) -> ProjectGraph {
    let resolver = Resolver::new(resolve_options());

    let mut graph = ProjectGraph::default();

    // TODO - ok so what I can do here is go over all the exports like the LLM wanted to,
    // but at the same time as we go over the components (and in parallel still) so that it's much
    // faster. And here we'll gather all the exports related to a file and map them to components
    let components: HashMap<ComponentKey, Component> = files
        .par_iter()
        .flat_map_iter(|file_info| {
            file_info.components.iter().map(|component_def| {
                let normalized_file_path = normalize_file_path_string(&file_info.file_path);

                let component = Component::new(&normalized_file_path, &component_def.name);

                (component.key.clone(), component)
            })
        })
        .collect();

    println!("all components: {:?}\n\n\n\n\n", components);

    // We'd like to do this in parallel but we're going to have to work around sharing the hashmap
    for file_info in files {
        for edge in &file_info.render_edges {
            let current_file_path = normalize_file_path_string(&file_info.file_path);
            println!("current_file_path: {:?}", current_file_path);
            let parent_component = components.get(&edge.parent_component_key);

            println!("parent component: {:?}", parent_component);

            if parent_component.is_none() {
                // TODO error handling in general hehe :)
                continue;
            }

            if let Some(child_component) = resolve_child_component(
                file_info,
                &edge.child_component_name,
                &resolver,
                current_file_path,
            ) {
                let _ = graph.resolved_render_edges.insert(
                    child_component.key.clone(),
                    ResolvedRenderEdge {
                        parent_component: parent_component.unwrap().clone(),
                        child_component: child_component,
                        span: edge.span,
                    },
                );
            }
        }
    }

    graph.components = components
        .into_iter()
        .map(|(_, component)| component)
        .collect();

    graph
}

fn resolve_child_component(
    file_info: &FileInfo,
    child_symbol: &str,
    resolver: &Resolver,
    current_file_path: String,
) -> Option<Component> {
    let import_symbol = file_info.module_imports.iter().find(|import_symbol| {
        import_symbol.local_name == child_symbol && !import_symbol.is_type_only
    })?;

    let resolve_result = resolver
        .resolve_file(Path::new(&current_file_path), &import_symbol.source_module)
        .ok()?;

    let resolved_file_path = normalize_file_path_from_path(resolve_result.path());

    // Yeah ok so file_info here is the wrong file, it's the parent file
    // Also import symbol is wrong
    let export_alias = export_alias(&import_symbol.source_module, file_info);

    println!("resolved_file_path: {:?}", resolved_file_path);
    println!("export_alias: {:?}", export_alias);

    if let Some(export_alias) = export_alias {
        return Some(Component::new(&resolved_file_path, &export_alias));
    }

    match import_symbol.kind {
        // TODO: handle default imports properly
        ImportKind::Default => None,
        ImportKind::Named => {
            let imported_name = import_symbol
                .imported_name
                .as_deref()
                .unwrap_or(&import_symbol.local_name);
            Some(Component::new(&resolved_file_path, imported_name))
        }
        ImportKind::Namespace => None,
    }
}

fn export_alias(component_name: &String, file_info: &FileInfo) -> Option<String> {
    println!("module_exports: {:?}", file_info.module_exports);

    file_info
        .module_exports
        .iter()
        .find(|export_symbol| {
            println!("export_symbol: {:?}", export_symbol);
            if export_symbol.is_type_only {
                return false;
            }

            if export_symbol.source_module.is_some() {
                return false;
            }

            if export_symbol.kind == ExportKind::ReExportAll {
                return false;
            }

            if let Some(local_name) = &export_symbol.local_name {
                println!("local_name: {:?}", local_name);
                println!("component_name: {:?}", component_name);
                return local_name == component_name;
            } else {
                return false;
            }
        })
        .and_then(|export_symbol| export_symbol.local_name.clone())
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
