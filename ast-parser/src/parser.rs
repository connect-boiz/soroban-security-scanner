use syn::{visit::{self, Visit}, ItemImpl, ItemStruct, ItemEnum, ItemTrait, Attribute, Meta, Result};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SorobanContractInfo {
    pub contract_impls: Vec<ContractImplInfo>,
    pub contract_types: Vec<ContractTypeInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractImplInfo {
    pub name: String,
    pub functions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractTypeInfo {
    pub name: String,
    pub kind: String, // "struct" or "enum"
}

pub struct SorobanVisitor {
    info: SorobanContractInfo,
}

impl SorobanVisitor {
    pub fn new() -> Self {
        Self {
            info: SorobanContractInfo::default(),
        }
    }

    pub fn finish(self) -> SorobanContractInfo {
        self.info
    }

    fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
        attrs.iter().any(|attr| {
            if let Meta::Path(path) = &attr.meta {
                path.is_ident(name)
            } else {
                false
            }
        })
    }
}

impl<'ast> Visit<'ast> for SorobanVisitor {
    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if Self::has_attribute(&i.attrs, "contractimpl") {
            let name = if let Some((_, path, _)) = &i.trait_ {
                path.segments.last().unwrap().ident.to_string()
            } else if let syn::Type::Path(tp) = &*i.self_ty {
                tp.path.segments.last().unwrap().ident.to_string()
            } else {
                "Unknown".to_string()
            };

            let mut functions = Vec::new();
            for item in &i.items {
                if let syn::ImplItem::Fn(f) = item {
                    functions.push(f.sig.ident.to_string());
                }
            }

            self.info.contract_impls.push(ContractImplInfo {
                name,
                functions,
            });
        }
        // Continue visiting children
        visit::visit_item_impl(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if Self::has_attribute(&i.attrs, "contracttype") {
            self.info.contract_types.push(ContractTypeInfo {
                name: i.ident.to_string(),
                kind: "struct".to_string(),
            });
        }
        visit::visit_item_struct(self, i);
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        if Self::has_attribute(&i.attrs, "contracttype") {
            self.info.contract_types.push(ContractTypeInfo {
                name: i.ident.to_string(),
                kind: "enum".to_string(),
            });
        }
        visit::visit_item_enum(self, i);
    }
}

pub fn parse_soroban_code(code: &str) -> Result<SorobanContractInfo> {
    let file = syn::parse_file(code)?;
    let mut visitor = SorobanVisitor::new();
    visitor.visit_file(&file);
    Ok(visitor.finish())
}

pub fn normalize_code(code: &str) -> String {
    // Basic normalization: strip comments using regex
    let re_block = regex::Regex::new(r"/\*[\s\S]*?\*/").unwrap();
    let re_line = regex::Regex::new(r"//.*").unwrap();
    
    let stripped = re_block.replace_all(code, "");
    let stripped = re_line.replace_all(&stripped, "");
    
    // Standardize whitespace (basic)
    stripped.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_contract_impl() {
        let code = r#"
            #[contractimpl]
            impl MyContract {
                pub fn hello() {}
            }
        "#;
        let info = parse_soroban_code(code).unwrap();
        assert_eq!(info.contract_impls.len(), 1);
        assert_eq!(info.contract_impls[0].name, "MyContract");
        assert!(info.contract_impls[0].functions.contains(&"hello".to_string()));
    }

    #[test]
    fn test_parse_contract_type() {
        let code = r#"
            #[contracttype]
            pub struct MyData {
                pub value: u32,
            }
        "#;
        let info = parse_soroban_code(code).unwrap();
        assert_eq!(info.contract_types.len(), 1);
        assert_eq!(info.contract_types[0].name, "MyData");
    }

    #[test]
    fn test_normalize_code() {
        let code = "// comment\n#[contractimpl]\nimpl C {}";
        let normalized = normalize_code(code);
        assert!(!normalized.contains("// comment"));
        assert!(normalized.contains("#[contractimpl]"));
    }
}
