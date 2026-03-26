use std::{collections::BTreeSet, path::PathBuf};

use context_analyzer_engine::collect::collect_project_info;
use context_analyzer_frontend::load_source_files;

#[test]
fn linker_resolves_named_import_render_edge() {
    let fixture_input = fixture_input_path("link_resolves_named_import");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    println!("{:?}", project_info.graph.resolved_render_edges);

    let app_children = project_info.graph.resolved_render_edges[0].clone();

    assert_eq!(app_children.len(), 1);
    assert_eq!(app_children[0].parent_component_id, 0);
    assert_eq!(app_children[0].child_component_id, 1);

    let child_component = project_info.graph.components[app_children[0].child_component_id].clone();
    let parent_component =
        project_info.graph.components[app_children[0].parent_component_id].clone();
    assert_eq!(parent_component.name, "App");
    assert_eq!(child_component.name, "ProfilePage");
}

#[test]
fn linker_resolves_member_expression_edge() {
    let fixture_input = fixture_input_path("link_resolves_member_expression");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    let app_children = project_info.graph.resolved_render_edges[0].clone();
    assert_eq!(app_children.len(), 1);
    assert_eq!(app_children[0].parent_component_id, 0);
    assert_eq!(app_children[0].child_component_id, 1);

    let child_component = project_info.graph.components[app_children[0].child_component_id].clone();
    let parent_component =
        project_info.graph.components[app_children[0].parent_component_id].clone();
    assert_eq!(parent_component.name, "App");
    assert_eq!(child_component.name, "Button");
}

#[test]
fn linker_resolves_named_import_via_export_alias() {
    let fixture_input = fixture_input_path("link_resolves_export_alias");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    let app_children = project_info.graph.resolved_render_edges[0].clone();
    assert_eq!(app_children.len(), 1);
    assert_eq!(app_children[0].parent_component_id, 0);
    assert_eq!(app_children[0].child_component_id, 1);

    let child_component = project_info.graph.components[app_children[0].child_component_id].clone();
    let parent_component =
        project_info.graph.components[app_children[0].parent_component_id].clone();
    assert_eq!(parent_component.name, "App");
    assert_eq!(child_component.name, "InternalProfilePage");
}

#[test]
fn linker_resolves_default_export() {
    let fixture_input = fixture_input_path("link_resolves_default_export");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    let app_children = project_info.graph.resolved_render_edges[0].clone();
    assert_eq!(app_children.len(), 1);
    assert_eq!(app_children[0].parent_component_id, 0);
    assert_eq!(app_children[0].child_component_id, 1);

    let child_component = project_info.graph.components[app_children[0].child_component_id].clone();
    let parent_component =
        project_info.graph.components[app_children[0].parent_component_id].clone();
    assert_eq!(parent_component.name, "App");
    assert_eq!(child_component.name, "ProfilePage");
}

#[test]
fn linker_resolves_nested_children_and_tracks_parent_jsx_symbol() {
    let fixture_input = fixture_input_path("link_resolves_nested_children");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    let resolved_pairs: BTreeSet<(String, String)> = project_info
        .graph
        .resolved_render_edges
        .iter()
        .flatten()
        .map(|edge| {
            let parent_name = project_info.graph.components[edge.parent_component_id]
                .name
                .clone();
            let child_name = project_info.graph.components[edge.child_component_id]
                .name
                .clone();
            (parent_name, child_name)
        })
        .collect();

    let expected_pairs = BTreeSet::from([
        ("App".to_string(), "PageShell".to_string()),
        ("App".to_string(), "ProfilePage".to_string()),
        ("PageShell".to_string(), "GlobalNav".to_string()),
        ("ProfilePage".to_string(), "Avatar".to_string()),
    ]);

    assert_eq!(resolved_pairs, expected_pairs);

    assert!(!project_info
        .graph
        .resolved_render_edges
        .iter()
        .flatten()
        .any(|edge| {
            let child_name = &project_info.graph.components[edge.child_component_id].name;
            child_name == "LocalBadge" || child_name == "ShellFrame"
        }));

    let app_file = project_info
        .files
        .iter()
        .find(|file| file.file_path.ends_with("/App.tsx") || file.file_path.ends_with("\\App.tsx"))
        .expect("expected fixture to include App.tsx file info");

    assert!(app_file.unresolved_render_edges.iter().any(|edge| {
        edge.parent_component_name == "App"
            && edge.child_rendered_symbol == "ProfilePage"
            && edge.parent_jsx_symbol == "PageShell"
    }));
}

#[test]
fn linker_finds_multiple_distinct_parents_for_shared_child() {
    let fixture_input = fixture_input_path("link_shared_child_multiple_distinct_parents");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    let resolved_pairs: BTreeSet<(String, String)> = project_info
        .graph
        .resolved_render_edges
        .iter()
        .flatten()
        .map(|edge| {
            let parent_name = project_info.graph.components[edge.parent_component_id]
                .name
                .clone();
            let child_name = project_info.graph.components[edge.child_component_id]
                .name
                .clone();
            (parent_name, child_name)
        })
        .collect();

    let expected_pairs = BTreeSet::from([
        ("App".to_string(), "LeftPane".to_string()),
        ("App".to_string(), "RightPane".to_string()),
        ("LeftPane".to_string(), "SharedChild".to_string()),
        ("RightPane".to_string(), "SharedChild".to_string()),
    ]);
    assert_eq!(resolved_pairs, expected_pairs);

    let shared_child_parents: BTreeSet<String> = resolved_pairs
        .iter()
        .filter(|(_, child)| child == "SharedChild")
        .map(|(parent, _)| parent.clone())
        .collect();
    assert_eq!(
        shared_child_parents,
        BTreeSet::from(["LeftPane".to_string(), "RightPane".to_string()])
    );

    let shared_child_edge_count = project_info
        .graph
        .resolved_render_edges
        .iter()
        .flatten()
        .filter(|edge| project_info.graph.components[edge.child_component_id].name == "SharedChild")
        .count();
    assert_eq!(shared_child_edge_count, 2);
}

fn fixture_input_path(fixture_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(fixture_name)
        .join("input")
}
