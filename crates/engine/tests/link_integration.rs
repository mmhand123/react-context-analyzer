use std::path::PathBuf;

use context_analyzer_engine::collect::collect_project_info;
use context_analyzer_frontend::load_source_files;

#[test]
fn linker_resolves_named_import_render_edge() {
    let fixture_input = fixture_input_path("link_resolves_named_import");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    println!("{:?}", project_info.graph.resolved_render_edges);

    assert!(
        project_info
            .graph
            .resolved_render_edges
            .iter()
            .any(|(_, edge)| {
                println!("{:?}", edge);

                edge.parent_component.name == "App" && edge.child_component.name == "ProfilePage"
            })
    );
}

#[test]
fn linker_resolves_member_expression_edge() {
    let fixture_input = fixture_input_path("link_resolves_member_expression");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    assert!(
        project_info
            .graph
            .resolved_render_edges
            .iter()
            .any(|(_, edge)| {
                edge.parent_component.name == "App" && edge.child_component.name == "Button"
            })
    );
}

#[test]
fn linker_resolves_named_import_via_export_alias() {
    let fixture_input = fixture_input_path("link_resolves_export_alias");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    assert!(
        project_info
            .graph
            .resolved_render_edges
            .iter()
            .any(|(_, edge)| {
                edge.parent_component.name == "App"
                    && edge.child_component.name == "InternalProfilePage"
            })
    );
}

#[test]
fn linker_resolves_default_export() {
    let fixture_input = fixture_input_path("link_resolves_default_export");
    let source_files =
        load_source_files(&fixture_input).expect("fixture source files should load cleanly");

    let project_info = collect_project_info(&source_files);

    assert!(
        project_info
            .graph
            .resolved_render_edges
            .iter()
            .any(|(_, edge)| {
                edge.parent_component.name == "App" && edge.child_component.name == "ProfilePage"
            })
    );
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
