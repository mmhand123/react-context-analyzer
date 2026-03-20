use crate::function_name::{
    classify_function_owner_kind, extract_component_name_from_function,
    extract_component_name_from_variable_declarator, extract_declared_function_name,
    extract_function_name_from_variable_declarator, extract_jsx_component_name,
};
use crate::link::build_project_graph;
use crate::paths::normalize_file_path_string;
use context_analyzer_core::model::{
    ComponentDef, ConsumerUse, ContextDef, ContextRef, ExportKind, ExportSymbol, FileInfo,
    FunctionOwnerKind, ImportKind, ImportSymbol, ProjectInfo, ProviderUse, RenderEdge,
    get_component_key,
};
use context_analyzer_frontend::SourceFileInput;
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, ExportAllDeclaration, ExportDefaultDeclaration,
    ExportDefaultDeclarationKind, ExportNamedDeclaration, Expression, Function, ImportDeclaration,
    ImportDeclarationSpecifier, JSXAttributeItem, JSXElement, JSXElementName, JSXOpeningElement,
    ModuleExportName, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;
use rayon::prelude::*;
use std::path::Path;

pub fn collect_project_info(source_files: &[SourceFileInput]) -> ProjectInfo {
    let files: Vec<FileInfo> = source_files.par_iter().map(collect_file_info).collect();
    let graph = build_project_graph(&files);

    let mut project_info = ProjectInfo::from_files(files);
    project_info.graph = graph;
    project_info
}

pub fn collect_file_info(source_file: &SourceFileInput) -> FileInfo {
    let source_type = match SourceType::from_path(Path::new(&source_file.path)) {
        Ok(source_type) => source_type,
        Err(_) => return empty_file_info(&source_file.path),
    };

    let allocator = Allocator::default();
    let parser_output = Parser::new(&allocator, &source_file.source_text, source_type).parse();

    if !parser_output.errors.is_empty() {
        return empty_file_info(&source_file.path);
    }

    let file_path = source_file.path.to_string_lossy().to_string();

    let mut collector = AstCollector::new(&file_path);
    collector.visit_program(&parser_output.program);

    FileInfo {
        file_path: file_path,
        contexts: collector.contexts,
        components: collector.components,
        module_imports: collector.module_imports,
        module_exports: collector.module_exports,
        providers: collector.providers,
        consumers: collector.consumers,
        render_edges: collector.render_edges,
    }
}

fn empty_file_info(file_path: &std::path::Path) -> FileInfo {
    FileInfo {
        file_path: file_path.to_string_lossy().to_string(),
        contexts: Vec::new(),
        components: Vec::new(),
        module_imports: Vec::new(),
        module_exports: Vec::new(),
        providers: Vec::new(),
        consumers: Vec::new(),
        render_edges: Vec::new(),
    }
}

#[derive(Default)]
struct AstCollector {
    contexts: Vec<ContextDef>,
    components: Vec<ComponentDef>,
    module_imports: Vec<ImportSymbol>,
    module_exports: Vec<ExportSymbol>,
    providers: Vec<ProviderUse>,
    consumers: Vec<ConsumerUse>,
    render_edges: Vec<RenderEdge>,
    component_stack: Vec<String>,
    function_owner_stack: Vec<FunctionOwnerFrame>,
    file_path: String,
}

impl AstCollector {
    fn new(file_path: &str) -> Self {
        Self {
            file_path: normalize_file_path_string(file_path),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
struct FunctionOwnerFrame {
    name: String,
    kind: FunctionOwnerKind,
}

impl<'a> Visit<'a> for AstCollector {
    fn visit_import_declaration(&mut self, import_declaration: &ImportDeclaration<'a>) {
        let source_module = import_declaration.source.value.as_str().to_string();

        if let Some(specifiers) = &import_declaration.specifiers {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                        self.module_imports.push(ImportSymbol {
                            source_module: source_module.clone(),
                            local_name: import_specifier.local.name.as_str().to_string(),
                            imported_name: Some(module_export_name(
                                import_specifier.imported.clone(),
                            )),
                            kind: ImportKind::Named,
                            is_type_only: import_specifier.import_kind.is_type(),
                        });
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(
                        import_default_specifier,
                    ) => {
                        self.module_imports.push(ImportSymbol {
                            source_module: source_module.clone(),
                            local_name: import_default_specifier.local.name.as_str().to_string(),
                            imported_name: Some("default".to_string()),
                            kind: ImportKind::Default,
                            is_type_only: false,
                        });
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                        import_namespace_specifier,
                    ) => {
                        self.module_imports.push(ImportSymbol {
                            source_module: source_module.clone(),
                            local_name: import_namespace_specifier.local.name.as_str().to_string(),
                            imported_name: None,
                            kind: ImportKind::Namespace,
                            is_type_only: false,
                        });
                    }
                }
            }
        }

        walk::walk_import_declaration(self, import_declaration);
    }

    fn visit_export_named_declaration(
        &mut self,
        export_named_declaration: &ExportNamedDeclaration<'a>,
    ) {
        let source_module = export_named_declaration
            .source
            .as_ref()
            .map(|literal| literal.value.as_str().to_string());

        for specifier in &export_named_declaration.specifiers {
            self.module_exports.push(ExportSymbol {
                export_name: module_export_name(specifier.exported.clone()),
                local_name: Some(module_export_name(specifier.local.clone())),
                source_module: source_module.clone(),
                kind: ExportKind::Named,
                is_type_only: specifier.export_kind.is_type(),
            });
        }

        walk::walk_export_named_declaration(self, export_named_declaration);
    }

    fn visit_export_default_declaration(
        &mut self,
        export_default_declaration: &ExportDefaultDeclaration<'a>,
    ) {
        let local_name = match &export_default_declaration.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function_declaration) => {
                function_declaration
                    .id
                    .as_ref()
                    .map(|identifier| identifier.name.as_str().to_string())
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class_declaration) => class_declaration
                .id
                .as_ref()
                .map(|identifier| identifier.name.as_str().to_string()),
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => None,
            _ => None,
        };

        self.module_exports.push(ExportSymbol {
            export_name: "default".to_string(),
            local_name,
            source_module: None,
            kind: ExportKind::Default,
            is_type_only: false,
        });

        walk::walk_export_default_declaration(self, export_default_declaration);
    }

    fn visit_export_all_declaration(&mut self, export_all_declaration: &ExportAllDeclaration<'a>) {
        self.module_exports.push(ExportSymbol {
            export_name: "*".to_string(),
            local_name: None,
            source_module: Some(export_all_declaration.source.value.as_str().to_string()),
            kind: ExportKind::ReExportAll,
            is_type_only: export_all_declaration.export_kind.is_type(),
        });

        walk::walk_export_all_declaration(self, export_all_declaration);
    }

    fn visit_function(&mut self, function_node: &Function<'a>, flags: ScopeFlags) {
        if let Some(function_name) = extract_declared_function_name(function_node) {
            let owner_kind = classify_function_owner_kind(&function_name);

            if owner_kind == FunctionOwnerKind::Component {
                self.components.push(ComponentDef {
                    name: function_name.clone(),
                    key: get_component_key(&self.file_path, &function_name),
                    span: function_node.span,
                });
                self.component_stack.push(function_name.clone());
            }

            self.function_owner_stack.push(FunctionOwnerFrame {
                name: function_name,
                kind: owner_kind.clone(),
            });

            walk::walk_function(self, function_node, flags);

            self.function_owner_stack.pop();
            if owner_kind == FunctionOwnerKind::Component {
                self.component_stack.pop();
            }
            return;
        }

        if let Some(component_name) = extract_component_name_from_function(function_node) {
            self.components.push(ComponentDef {
                name: component_name.clone(),
                key: get_component_key(&self.file_path, &component_name),
                span: function_node.span,
            });
            self.component_stack.push(component_name);
            walk::walk_function(self, function_node, flags);
            self.component_stack.pop();
        } else {
            walk::walk_function(self, function_node, flags);
        }
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let Some(binding_identifier) = declarator.id.get_binding_identifier() else {
            walk::walk_variable_declarator(self, declarator);
            return;
        };

        let identifier_name = binding_identifier.name.as_str();

        if let Some(initializer_expression) = declarator.init.as_ref() {
            if is_create_context_call(initializer_expression) {
                self.contexts.push(ContextDef {
                    name: identifier_name.to_string(),
                    span: binding_identifier.span,
                });
            }

            if let Some(component_name) =
                extract_component_name_from_variable_declarator(declarator)
            {
                self.components.push(ComponentDef {
                    name: component_name.clone(),
                    key: get_component_key(&self.file_path, &component_name),
                    span: binding_identifier.span,
                });
                self.component_stack.push(component_name);
                self.function_owner_stack.push(FunctionOwnerFrame {
                    name: identifier_name.to_string(),
                    kind: FunctionOwnerKind::Component,
                });
                walk::walk_expression(self, initializer_expression);
                self.function_owner_stack.pop();
                self.component_stack.pop();
                return;
            }

            if let Some(function_name) = extract_function_name_from_variable_declarator(declarator)
            {
                let owner_kind = classify_function_owner_kind(&function_name);
                self.function_owner_stack.push(FunctionOwnerFrame {
                    name: function_name,
                    kind: owner_kind,
                });
                walk::walk_expression(self, initializer_expression);
                self.function_owner_stack.pop();
                return;
            }
        }

        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_call_expression(&mut self, call_expression: &CallExpression<'a>) {
        if let Some(context_symbol) = extract_context_symbol_from_consumer_call(call_expression) {
            self.consumers.push(ConsumerUse {
                context_ref: ContextRef {
                    symbol: context_symbol,
                    resolved_context_id: None,
                },
                containing_component_name: self.component_stack.last().cloned(),
                containing_function_name: self.current_function_owner_name(),
                containing_function_kind: self.current_function_owner_kind(),
                span: call_expression.span,
            });
        }

        walk::walk_call_expression(self, call_expression);
    }

    fn visit_jsx_element(&mut self, jsx_element: &JSXElement<'a>) {
        let opening_element = &jsx_element.opening_element;

        if let Some(provider_symbol) = extract_provider_symbol(opening_element.as_ref()) {
            self.providers.push(ProviderUse {
                context_ref: ContextRef {
                    symbol: provider_symbol,
                    resolved_context_id: None,
                },
                containing_component_name: self.component_stack.last().cloned(),
                containing_function_name: self.current_function_owner_name(),
                containing_function_kind: self.current_function_owner_kind(),
                span: opening_element.span,
            });
        }

        if let Some(current_component_name) = self.component_stack.last()
            && let Some(child_component_name) = extract_jsx_component_name(&opening_element.name)
            && child_component_name != *current_component_name
        {
            self.render_edges.push(RenderEdge {
                parent_component_key: get_component_key(&self.file_path, current_component_name),
                parent_component_name: current_component_name.clone(),
                child_component_name,
                span: opening_element.span,
            });
        }

        walk::walk_jsx_element(self, jsx_element);
    }
}

impl AstCollector {
    fn current_function_owner_name(&self) -> Option<String> {
        self.function_owner_stack
            .last()
            .map(|owner| owner.name.clone())
    }

    fn current_function_owner_kind(&self) -> Option<FunctionOwnerKind> {
        self.function_owner_stack
            .last()
            .map(|owner| owner.kind.clone())
    }
}

fn is_create_context_call(expression: &Expression<'_>) -> bool {
    let Expression::CallExpression(call_expression) = expression else {
        return false;
    };

    match &call_expression.callee {
        Expression::Identifier(identifier) => identifier.name.as_str() == "createContext",
        Expression::StaticMemberExpression(member_expression) => member_expression
            .object
            .get_identifier_reference()
            .is_some_and(|identifier| {
                identifier.name.as_str() == "React"
                    && member_expression.property.name.as_str() == "createContext"
            }),
        _ => false,
    }
}

fn extract_context_symbol_from_consumer_call(
    call_expression: &CallExpression<'_>,
) -> Option<String> {
    let is_consumer_call = match &call_expression.callee {
        Expression::Identifier(identifier) => {
            let name = identifier.name.as_str();
            name == "useContext" || name == "use"
        }
        Expression::StaticMemberExpression(member_expression) => member_expression
            .object
            .get_identifier_reference()
            .is_some_and(|identifier| {
                identifier.name.as_str() == "React"
                    && member_expression.property.name.as_str() == "useContext"
            }),
        _ => false,
    };

    if !is_consumer_call {
        return None;
    }

    let first_argument = call_expression.arguments.first()?;
    argument_identifier_name(first_argument)
}

fn argument_identifier_name(argument: &Argument<'_>) -> Option<String> {
    match argument {
        Argument::Identifier(identifier) => Some(identifier.name.as_str().to_string()),
        _ => None,
    }
}

fn extract_provider_symbol(opening_element: &JSXOpeningElement<'_>) -> Option<String> {
    match &opening_element.name {
        JSXElementName::Identifier(identifier) => {
            let name = identifier.name.as_str();
            if name.ends_with("Context") && jsx_has_value_attribute(opening_element) {
                Some(name.to_string())
            } else {
                None
            }
        }
        JSXElementName::IdentifierReference(identifier) => {
            let name = identifier.name.as_str();
            if name.ends_with("Context") && jsx_has_value_attribute(opening_element) {
                Some(name.to_string())
            } else {
                None
            }
        }
        JSXElementName::MemberExpression(member_expression) => {
            let property_name = member_expression.property.name.as_str();
            if property_name != "Provider" {
                return None;
            }

            match &member_expression.object {
                oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => {
                    Some(identifier.name.as_str().to_string())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn module_export_name(export_name: ModuleExportName<'_>) -> String {
    match export_name {
        ModuleExportName::IdentifierName(identifier_name) => {
            identifier_name.name.as_str().to_string()
        }
        ModuleExportName::IdentifierReference(identifier_reference) => {
            identifier_reference.name.as_str().to_string()
        }
        ModuleExportName::StringLiteral(string_literal) => {
            string_literal.value.as_str().to_string()
        }
    }
}

fn jsx_has_value_attribute(opening_element: &JSXOpeningElement<'_>) -> bool {
    opening_element
        .attributes
        .iter()
        .any(|attribute| match attribute {
            JSXAttributeItem::Attribute(attribute) => attribute
                .name
                .as_identifier()
                .is_some_and(|identifier| identifier.name.as_str() == "value"),
            JSXAttributeItem::SpreadAttribute(_) => false,
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use context_analyzer_core::model::{ExportKind, FunctionOwnerKind, ImportKind};
    use context_analyzer_frontend::SourceFileInput;

    use super::{collect_file_info, collect_project_info};

    #[test]
    fn collect_file_info_extracts_context_component_provider_consumer_and_render_edges() {
        let source_file = SourceFileInput {
            path: PathBuf::from("src/App.tsx"),
            source_text: r#"
                import React, { createContext, useContext } from "react";

                const AuthContext = createContext(null);

                function App() {
                    const auth = useContext(AuthContext);
                    return (
                        <AuthContext.Provider value={auth}>
                            <ProfilePage />
                        </AuthContext.Provider>
                    );
                }

                const ProfilePage = () => {
                    return <Header />;
                };
            "#
            .to_string(),
        };

        let file_info = collect_file_info(&source_file);

        assert_eq!(file_info.contexts.len(), 1);
        assert_eq!(file_info.contexts[0].name, "AuthContext");

        assert_eq!(file_info.components.len(), 2);
        assert!(
            file_info
                .components
                .iter()
                .any(|component| component.name == "App")
        );
        assert!(
            file_info
                .components
                .iter()
                .any(|component| component.name == "ProfilePage")
        );

        assert_eq!(file_info.providers.len(), 1);
        assert_eq!(file_info.providers[0].context_ref.symbol, "AuthContext");
        assert_eq!(
            file_info.providers[0].containing_component_name,
            Some("App".to_string())
        );

        assert_eq!(file_info.consumers.len(), 1);
        assert_eq!(file_info.consumers[0].context_ref.symbol, "AuthContext");
        assert_eq!(
            file_info.consumers[0].containing_component_name,
            Some("App".to_string())
        );

        assert_eq!(file_info.render_edges.len(), 3);
        assert!(
            file_info
                .render_edges
                .iter()
                .any(|edge| edge.parent_component_name == "App"
                    && edge.child_component_name == "AuthContext.Provider")
        );
        assert!(
            file_info
                .render_edges
                .iter()
                .any(|edge| edge.parent_component_name == "App"
                    && edge.child_component_name == "ProfilePage")
        );
        assert!(
            file_info
                .render_edges
                .iter()
                .any(|edge| edge.parent_component_name == "ProfilePage"
                    && edge.child_component_name == "Header")
        );

        assert_eq!(file_info.module_imports.len(), 3);
        assert!(file_info.module_imports.iter().any(|import_symbol| {
            import_symbol.source_module == "react"
                && import_symbol.local_name == "React"
                && import_symbol.imported_name.as_deref() == Some("default")
                && import_symbol.kind == ImportKind::Default
        }));
        assert!(file_info.module_imports.iter().any(|import_symbol| {
            import_symbol.source_module == "react"
                && import_symbol.local_name == "createContext"
                && import_symbol.imported_name.as_deref() == Some("createContext")
                && import_symbol.kind == ImportKind::Named
        }));
        assert!(file_info.module_imports.iter().any(|import_symbol| {
            import_symbol.source_module == "react"
                && import_symbol.local_name == "useContext"
                && import_symbol.imported_name.as_deref() == Some("useContext")
                && import_symbol.kind == ImportKind::Named
        }));

        assert!(file_info.module_exports.is_empty());
    }

    #[test]
    fn collect_file_info_returns_empty_collections_for_parse_failures() {
        let source_file = SourceFileInput {
            path: PathBuf::from("src/Broken.tsx"),
            source_text: "const = ;".to_string(),
        };

        let file_info = collect_file_info(&source_file);

        assert!(file_info.contexts.is_empty());
        assert!(file_info.components.is_empty());
        assert!(file_info.module_imports.is_empty());
        assert!(file_info.module_exports.is_empty());
        assert!(file_info.providers.is_empty());
        assert!(file_info.consumers.is_empty());
        assert!(file_info.render_edges.is_empty());
    }

    #[test]
    fn collect_project_info_aggregates_counts_across_files() {
        let files = vec![
            SourceFileInput {
                path: PathBuf::from("src/App.tsx"),
                source_text: "function App() { return <ProfilePage />; }".to_string(),
            },
            SourceFileInput {
                path: PathBuf::from("src/Page.tsx"),
                source_text: "const ThemeContext = createContext(null); function ProfilePage() { const value = useContext(ThemeContext); return <div />; }".to_string(),
            },
        ];

        let project_info = collect_project_info(&files);

        assert_eq!(project_info.summary.file_count, 2);
        assert_eq!(project_info.summary.context_count, 1);
        assert_eq!(project_info.summary.component_count, 2);
        assert_eq!(project_info.summary.consumer_count, 1);
        assert_eq!(project_info.summary.render_edge_count, 1);
    }

    #[test]
    fn consumer_inside_hook_records_function_owner_mapping() {
        let source_file = SourceFileInput {
            path: PathBuf::from("src/hooks.tsx"),
            source_text: r#"
                import { useContext, createContext } from "react";

                const AuthContext = createContext(null);

                function useAuth() {
                    return useContext(AuthContext);
                }
            "#
            .to_string(),
        };

        let file_info = collect_file_info(&source_file);

        assert_eq!(file_info.consumers.len(), 1);
        assert_eq!(
            file_info.consumers[0].containing_function_name,
            Some("useAuth".to_string())
        );
        assert_eq!(
            file_info.consumers[0].containing_function_kind,
            Some(FunctionOwnerKind::Hook)
        );
        assert_eq!(file_info.consumers[0].containing_component_name, None);
    }

    #[test]
    fn collect_file_info_extracts_import_and_export_symbols() {
        let source_file = SourceFileInput {
            path: PathBuf::from("src/module.tsx"),
            source_text: r#"
                import DefaultPage, { ProfilePage as UserProfile } from "./ProfilePage";
                import * as UI from "./ui";

                export { UserProfile as Profile };
                export { Header } from "./Header";
                export * from "./shared";
                export default function ModulePage() {
                    return <DefaultPage />;
                }
            "#
            .to_string(),
        };

        let file_info = collect_file_info(&source_file);

        assert_eq!(file_info.module_imports.len(), 3);
        assert!(file_info.module_imports.iter().any(|import_symbol| {
            import_symbol.local_name == "DefaultPage"
                && import_symbol.kind == ImportKind::Default
                && import_symbol.source_module == "./ProfilePage"
        }));
        assert!(file_info.module_imports.iter().any(|import_symbol| {
            import_symbol.local_name == "UserProfile"
                && import_symbol.kind == ImportKind::Named
                && import_symbol.imported_name.as_deref() == Some("ProfilePage")
        }));
        assert!(file_info.module_imports.iter().any(|import_symbol| {
            import_symbol.local_name == "UI"
                && import_symbol.kind == ImportKind::Namespace
                && import_symbol.source_module == "./ui"
        }));

        assert_eq!(file_info.module_exports.len(), 4);
        assert!(file_info.module_exports.iter().any(|export_symbol| {
            export_symbol.export_name == "Profile"
                && export_symbol.local_name.as_deref() == Some("UserProfile")
                && export_symbol.kind == ExportKind::Named
                && export_symbol.source_module.is_none()
        }));
        assert!(file_info.module_exports.iter().any(|export_symbol| {
            export_symbol.export_name == "Header"
                && export_symbol.local_name.as_deref() == Some("Header")
                && export_symbol.kind == ExportKind::Named
                && export_symbol.source_module.as_deref() == Some("./Header")
        }));
        assert!(file_info.module_exports.iter().any(|export_symbol| {
            export_symbol.export_name == "*"
                && export_symbol.kind == ExportKind::ReExportAll
                && export_symbol.source_module.as_deref() == Some("./shared")
        }));
        assert!(file_info.module_exports.iter().any(|export_symbol| {
            export_symbol.export_name == "default" && export_symbol.kind == ExportKind::Default
        }));
    }
}
