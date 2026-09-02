use std::fs;

use super::model::{Generated, generate_enum, generate_struct};
use super::output::{
    GENERATED_HEADER, LEGACY_PROJECT_TOOLS_GENERATED_HEADER, LEGACY_XTASK_GENERATED_HEADER,
    prepare_out_dir, render,
};
use super::{generate_declarations, run};

fn temporary_output_dir(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("test clock should be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ple-tsgen-{label}-{}-{nonce}", std::process::id()))
}

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        Self(temporary_output_dir(label))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn output_cleanup_removes_stale_owned_types_but_refuses_authored_types() {
    let out_dir = TestDirectory::new("cleanup");
    fs::create_dir_all(out_dir.path()).expect("temporary output should be created");
    let notes = out_dir.path().join("Notes.txt");
    fs::write(&notes, "preserve me\n").expect("non-TypeScript output should be written");
    let stale = out_dir.path().join("Stale.ts");
    fs::write(
        &stale,
        format!("{GENERATED_HEADER}\nexport type Stale = string;\n"),
    )
    .expect("stale generated type should be written");
    prepare_out_dir(out_dir.path()).expect("owned stale output should be removable");
    assert!(!stale.exists());
    assert!(notes.exists());
    let legacy_project_tools = out_dir.path().join("LegacyProjectTools.ts");
    fs::write(
        &legacy_project_tools,
        format!(
            "{LEGACY_PROJECT_TOOLS_GENERATED_HEADER}\nexport type LegacyProjectTools = string;\n"
        ),
    )
    .expect("legacy generated type should be written");
    prepare_out_dir(out_dir.path()).expect("exact legacy output should be migrated");
    assert!(!legacy_project_tools.exists());
    assert!(notes.exists());
    let legacy_xtask = out_dir.path().join("LegacyXtask.ts");
    fs::write(
        &legacy_xtask,
        format!("{LEGACY_XTASK_GENERATED_HEADER}\nexport type LegacyXtask = string;\n"),
    )
    .expect("legacy xtask generated type should be written");
    prepare_out_dir(out_dir.path()).expect("exact xtask output should be migrated");
    assert!(!legacy_xtask.exists());
    assert!(notes.exists());
    let stale_content = format!("{GENERATED_HEADER}\nexport type Stale = string;\n");
    fs::write(&stale, &stale_content).expect("owned stale output should be restored");
    let authored = out_dir.path().join("Authored.ts");
    let authored_content = "export type Authored = string;\n";
    fs::write(&authored, authored_content).expect("authored fixture should be written");
    assert!(prepare_out_dir(out_dir.path()).is_err());
    assert_eq!(
        fs::read_to_string(&stale).expect("owned stale output should remain readable"),
        stale_content
    );
    assert_eq!(
        fs::read_to_string(&authored).expect("authored output should remain readable"),
        authored_content
    );
    assert!(notes.exists());
    fs::remove_file(&authored).expect("authored fixture should be removed before spoof check");
    let spoofed = out_dir.path().join("Spoofed.ts");
    let spoofed_content = format!("{GENERATED_HEADER} but not the exact ownership marker\n");
    fs::write(&spoofed, &spoofed_content).expect("spoofed marker fixture should be written");
    assert!(prepare_out_dir(out_dir.path()).is_err());
    assert_eq!(
        fs::read_to_string(&stale).expect("owned stale output should remain readable"),
        stale_content
    );
    assert_eq!(
        fs::read_to_string(&spoofed).expect("spoofed output should remain readable"),
        spoofed_content
    );
    assert!(notes.exists());
}

#[test]
fn duplicate_public_names_fail_before_owned_cleanup() {
    let root_a = TestDirectory::new("duplicate-root-a");
    let root_b = TestDirectory::new("duplicate-root-b");
    let out_dir = TestDirectory::new("duplicate-output");
    fs::create_dir_all(root_a.path()).expect("first temporary source should be created");
    fs::create_dir_all(root_b.path()).expect("second temporary source should be created");
    fs::create_dir_all(out_dir.path()).expect("temporary output should be created");
    let first_origin = root_a.path().join("first.rs");
    let second_origin = root_b.path().join("second.rs");
    fs::write(
        &first_origin,
        "#[derive(Serialize)]\npub struct SharedContract { pub value: String }\n",
    )
    .expect("first serializable source should be written");
    fs::write(
        &second_origin,
        "#[derive(Serialize)]\npub enum SharedContract { Available }\n",
    )
    .expect("second serializable source should be written");
    let stale = out_dir.path().join("Stale.ts");
    let stale_content = format!("{GENERATED_HEADER}\nexport type Stale = string;\n");
    fs::write(&stale, &stale_content).expect("owned stale output should be written");

    let declarations = generate_declarations(&[root_a.path(), root_b.path()])
        .expect("serializable source declarations should generate in memory");
    let error = super::source::declaration_names(&declarations)
        .expect_err("duplicate names should be refused");
    let message = error.to_string();
    assert!(message.contains("SharedContract"));
    assert!(message.contains(&first_origin.display().to_string()));
    assert!(message.contains(&second_origin.display().to_string()));
    assert_eq!(
        fs::read_to_string(&stale).expect("stale output should remain readable"),
        stale_content
    );
}

#[test]
fn direct_rendering_imports_only_generated_declarations() {
    let generated = Generated {
        name: "Outer".to_string(),
        dependencies: ["Nested", "Missing", "Outer", "Another"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        docs: Vec::new(),
        body: "Nested".to_string(),
    };
    let declaration_names = ["Nested", "Outer", "Another"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let rendered = render(&generated, &declaration_names);
    assert!(rendered.starts_with(GENERATED_HEADER));
    assert!(rendered.contains("import type { Another } from \"./Another\";"));
    assert!(rendered.contains("import type { Nested } from \"./Nested\";"));
    assert!(
        rendered
            .find("Another")
            .expect("another import should exist")
            < rendered.find("Nested").expect("nested import should exist")
    );
    assert!(!rendered.contains("Missing"));
    assert!(!rendered.contains("import type { Outer }"));
}

#[test]
fn explicit_contract_roots_generate_one_direct_type_graph() {
    let root_a = TestDirectory::new("cross-root-a");
    let root_b = TestDirectory::new("cross-root-b");
    let out_dir = TestDirectory::new("cross-root-output");
    fs::create_dir_all(root_a.path()).expect("first contract root should be created");
    fs::create_dir_all(root_b.path()).expect("second contract root should be created");
    fs::write(
        root_a.path().join("outer.rs"),
        "#[derive(Serialize)]\npub struct Outer { pub nested: Nested }\n",
    )
    .expect("outer source should be written");
    fs::write(
        root_b.path().join("nested.rs"),
        "#[derive(Serialize)]\npub struct Nested { pub value: String }\n",
    )
    .expect("nested source should be written");

    run(&[root_a.path(), root_b.path()], out_dir.path())
        .expect("explicit contract roots should generate a direct type graph");

    let outer = fs::read_to_string(out_dir.path().join("Outer.ts"))
        .expect("outer declaration should be generated");
    let nested = fs::read_to_string(out_dir.path().join("Nested.ts"))
        .expect("nested declaration should be generated");
    assert!(outer.starts_with(GENERATED_HEADER));
    assert!(nested.starts_with(GENERATED_HEADER));
    assert!(outer.contains("import type { Nested } from \"./Nested\";"));
    assert!(outer.contains("export type Outer = {"));
    assert!(nested.contains("export type Nested = {"));
    assert!(!out_dir.path().join("OuterCodec.ts").exists());
    assert!(!out_dir.path().join("index.ts").exists());
}

#[test]
fn hand_written_serde_wrappers_generate_their_browser_declaration() {
    let source = TestDirectory::new("manual-serde-wrapper");
    let out_dir = TestDirectory::new("manual-serde-wrapper-output");
    fs::create_dir_all(source.path()).expect("temporary source should be created");
    fs::write(
        source.path().join("wrapper.rs"),
        "pub struct BoundedItems(Vec<String>);\nimpl Serialize for BoundedItems { fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error> where S: serde::Serializer { unreachable!() } }\nimpl<'de> Deserialize<'de> for BoundedItems { fn deserialize<D>(_: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> { unreachable!() } }\n#[derive(Serialize)] pub struct BrowserRecord { pub items: BoundedItems }\n",
    )
    .expect("manual Serde wrapper source should be written");

    run(&[source.path()], out_dir.path()).expect("manual wrapper contract should generate");

    let wrapper = fs::read_to_string(out_dir.path().join("BoundedItems.ts"))
        .expect("manual wrapper declaration should be generated");
    let record = fs::read_to_string(out_dir.path().join("BrowserRecord.ts"))
        .expect("browser record declaration should be generated");
    assert!(wrapper.contains("export type BoundedItems = Array<string>;"));
    assert!(record.contains("import type { BoundedItems } from \"./BoundedItems\";"));
}

#[test]
fn empty_contract_roots_preserve_existing_output() {
    let out_dir = TestDirectory::new("empty-contract-roots");
    fs::create_dir_all(out_dir.path()).expect("temporary output should be created");
    let stale = out_dir.path().join("Stale.ts");
    let stale_content = format!("{GENERATED_HEADER}\nexport type Stale = string;\n");
    fs::write(&stale, &stale_content).expect("owned output sentinel should be written");

    let error = run(&[], out_dir.path()).expect_err("empty contract roots should be rejected");

    assert!(error.to_string().contains("at least one contract root"));
    assert_eq!(
        fs::read_to_string(&stale).expect("owned output sentinel should remain readable"),
        stale_content
    );
}
#[test]
fn unit_enums_become_string_unions() {
    let item: syn::ItemEnum = syn::parse_quote! { #[derive(Serialize)] #[serde(rename_all = "camelCase")] pub enum Colour { DeepRed, Blue, } };
    assert_eq!(
        generate_enum(&item)
            .expect("generation should succeed")
            .body,
        "\"deepRed\" | \"blue\""
    );
}
#[test]
fn kebab_case_enums_preserve_hyphenated_wire_identifiers() {
    let item: syn::ItemEnum = syn::parse_quote! { #[derive(Serialize)] #[serde(rename_all = "kebab-case")] pub enum Habitat { CoralReef, SaltMarsh, } };
    assert_eq!(
        generate_enum(&item)
            .expect("generation should support kebab-case")
            .body,
        "\"coral-reef\" | \"salt-marsh\""
    );
}
#[test]
fn public_u32_constants_become_safe_typescript_constants() {
    let source_dir = temporary_output_dir("u32-constant-source");
    let out_dir = temporary_output_dir("u32-constant-output");
    fs::create_dir_all(&source_dir).expect("temporary source directory should be created");
    fs::write(source_dir.join("timing.rs"), "/// A browser-safe whole Assignment Attempt time limit.\npub const DEFAULT_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS: u32 = 900;\n").expect("temporary source should be written");
    run(&[source_dir.as_path()], &out_dir).expect("u32 constant should generate");
    assert!(
        fs::read_to_string(out_dir.join("DEFAULT_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS.ts"))
            .expect("generated constant should be readable")
            .contains("export const DEFAULT_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS = 900 as const;")
    );
    fs::remove_dir_all(source_dir).expect("temporary source should be removed");
    fs::remove_dir_all(out_dir).expect("temporary output should be removed");
}
#[test]
fn option_maps_to_a_nullable_union() {
    let item: syn::ItemStruct =
        syn::parse_quote! { #[derive(Serialize)] pub struct Holder { pub label: Option<String>, } };
    assert!(
        generate_struct(&item)
            .expect("generation should succeed")
            .body
            .contains("label: string | null;")
    );
}
#[test]
fn empty_named_struct_maps_to_an_exact_empty_record() {
    let item: syn::ItemStruct =
        syn::parse_quote! { #[derive(Serialize)] pub struct EmptyRequest {} };
    let generated = generate_struct(&item).expect("generation should succeed");
    assert_eq!(generated.body, "Record<string, never>");
    assert!(generated.dependencies.is_empty());
}
#[test]
fn transparent_nonzero_integer_newtypes_use_their_numeric_wire_type() {
    let item: syn::ItemStruct = syn::parse_quote! { #[derive(Serialize)] #[serde(transparent)] pub struct PublicId(NonZeroU32); };
    let generated = generate_struct(&item).expect("generation should succeed");
    assert_eq!(generated.body, "number");
    assert!(generated.dependencies.is_empty());
}

#[test]
fn tagged_enum_flattened_struct_preserves_the_flat_wire_shape() {
    let item: syn::ItemEnum = syn::parse_quote! {
        #[derive(Serialize)]
        #[serde(tag = "backend", rename_all = "camelCase", rename_all_fields = "camelCase")]
        pub enum Locator {
            Imathas { #[serde(flatten)] binding: ImathasBinding },
        }
    };
    let generated = generate_enum(&item).expect("generation should support flattened bindings");
    assert!(generated.body.contains("backend: \"imathas\";"));
    assert!(generated.body.contains("& ImathasBinding"));
    assert!(!generated.body.contains("binding:"));
}
#[test]
fn omitted_option_becomes_an_optional_property() {
    let item: syn::ItemStruct = syn::parse_quote! { #[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct Holder { #[serde(skip_serializing_if = "Option::is_none")] pub secret: Option<String>, } };
    let generated = generate_struct(&item).expect("generation should succeed");
    assert!(generated.body.contains("secret?: string;"));
    assert!(!generated.body.contains("null"));
}
#[test]
fn nonserializable_server_contract_is_not_emitted_to_typescript() {
    let model_dir = temporary_output_dir("private-model");
    let out_dir = temporary_output_dir("private-output");
    fs::create_dir_all(&model_dir).expect("temporary model directory should be created");
    fs::write(
        model_dir.join("feedback.rs"),
        "pub struct QuestionFeedback { pub hidden: String }\n",
    )
    .expect("private feedback fixture should be written");
    run(&[model_dir.as_path()], &out_dir).expect("generation should accept private Rust types");
    assert!(!out_dir.join("QuestionFeedback.ts").exists());
    fs::remove_dir_all(model_dir).expect("temporary model directory should be removed");
    fs::remove_dir_all(out_dir).expect("temporary output directory should be removed");
}

#[test]
fn documented_server_held_contract_is_not_emitted_to_typescript() {
    let model_dir = TestDirectory::new("server-held-model");
    let out_dir = TestDirectory::new("server-held-output");
    fs::create_dir_all(model_dir.path()).expect("temporary model directory should be created");
    fs::write(
        model_dir.path().join("reproduction.rs"),
        "#[doc(hidden)]\n#[derive(Serialize)]\npub struct ReproductionDetails { pub secret: String }\n",
    )
    .expect("server-held reproduction fixture should be written");

    run(&[model_dir.path()], out_dir.path())
        .expect("generation should accept server-held contracts");

    assert!(!out_dir.path().join("ReproductionDetails.ts").exists());
}

#[test]
fn nested_production_modules_are_generated_but_test_modules_are_not() {
    let model_dir = temporary_output_dir("nested-model");
    let out_dir = temporary_output_dir("nested-output");
    let capability_dir = model_dir.join("presentation");
    let test_dir = capability_dir.join("tests");
    fs::create_dir_all(&test_dir).expect("temporary model directories should be created");
    fs::write(
        capability_dir.join("model.rs"),
        "#[derive(Serialize)]\npub struct BrowserContract { pub label: String }\n",
    )
    .expect("nested production fixture should be written");
    fs::write(
        capability_dir.join("tests.rs"),
        "#[derive(Serialize)]\npub struct FileTestFixture { pub secret: String }\n",
    )
    .expect("file test fixture should be written");
    fs::write(
        test_dir.join("fixture.rs"),
        "#[derive(Serialize)]\npub struct DirectoryTestFixture { pub secret: String }\n",
    )
    .expect("directory test fixture should be written");
    assert_eq!(
        run(&[model_dir.as_path()], &out_dir).expect("nested generation should succeed"),
        1
    );
    assert!(out_dir.join("BrowserContract.ts").exists());
    assert!(!out_dir.join("FileTestFixture.ts").exists());
    assert!(!out_dir.join("DirectoryTestFixture.ts").exists());
    fs::remove_dir_all(model_dir).expect("temporary model directory should be removed");
    fs::remove_dir_all(out_dir).expect("temporary output directory should be removed");
}
#[test]
fn an_externally_tagged_newtype_variant_preserves_scalar_siblings() {
    let item: syn::ItemEnum = syn::parse_quote! { #[derive(Serialize)] #[serde(rename_all = "camelCase")] pub enum Shape { Unavailable, Available(Statistics), } };
    let generated = generate_enum(&item).expect("generation should support external tagging");
    assert_eq!(
        generated.body,
        "\"unavailable\" | { available: Statistics }"
    );
    assert!(generated.dependencies.contains("Statistics"));
}
#[test]
fn tagged_enum_omitted_options_become_optional_properties() {
    let item: syn::ItemEnum = syn::parse_quote! { #[derive(Serialize)] #[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")] pub enum Evidence { Available { #[serde(skip_serializing_if = "Option::is_none")] discrimination_index: Option<f64>, }, } };
    let generated = generate_enum(&item).expect("generation should support omitted options");
    assert!(generated.body.contains("discriminationIndex?: number;"));
    assert!(
        !generated
            .body
            .contains("discriminationIndex: number | null;")
    );
}
#[test]
fn newtype_structs_alias_their_inner_type() {
    let item: syn::ItemStruct =
        syn::parse_quote! { #[derive(Serialize)] pub struct Tags(Vec<String>); };
    assert_eq!(
        generate_struct(&item)
            .expect("generation should succeed")
            .body,
        "Array<string>"
    );
}
#[test]
fn serde_string_newtypes_use_their_wire_type() {
    let item: syn::ItemStruct = syn::parse_quote! { #[derive(Serialize)] #[serde(try_from = "String", into = "String")] pub struct ExactDecimal(i64); };
    assert_eq!(
        generate_struct(&item)
            .expect("generation should succeed")
            .body,
        "string"
    );
}

#[test]
fn effective_serde_names_cover_fields_variants_and_literals() {
    let record: syn::ItemStruct = syn::parse_quote! {
        #[derive(Serialize)]
        #[serde(rename_all = "snake_case")]
        pub struct Record {
            pub regular_field: String,
            #[serde(rename = "direct-wire-name")]
            pub renamed_field: String,
        }
    };
    let record = generate_struct(&record).expect("literal field rename should generate");
    assert!(record.body.contains("regular_field: string;"));
    assert!(record.body.contains("\"direct-wire-name\": string;"));
    assert!(!record.body.contains("directWireName"));

    let choice: syn::ItemEnum = syn::parse_quote! {
        #[derive(Serialize)]
        #[serde(tag = "entry-kind", rename_all = "snake_case", rename_all_fields = "snake_case")]
        pub enum Choice {
            PlainChoice,
            #[serde(rename = "literal-\"choice")]
            NamedChoice { nested_value: String },
        }
    };
    let choice = generate_enum(&choice).expect("literal variant rename should generate");
    assert!(choice.body.contains("\"entry-kind\": \"plain_choice\""));
    assert!(
        choice
            .body
            .contains("\"entry-kind\": \"literal-\\\"choice\"")
    );
    assert!(choice.body.contains("nested_value: string;"));

    let external: syn::ItemEnum = syn::parse_quote! {
        #[derive(Serialize)]
        pub enum External {
            #[serde(rename = "external-kind")]
            Named(Payload),
        }
    };
    assert!(
        generate_enum(&external)
            .expect("external tag should generate")
            .body
            .contains("{ \"external-kind\": Payload }")
    );
}

#[test]
fn optional_nested_and_tagged_shapes_match_effective_serde() {
    let item: syn::ItemEnum = syn::parse_quote! {
        #[derive(Serialize)]
        #[serde(tag = "state_kind", rename_all = "snake_case", rename_all_fields = "snake_case")]
        pub enum Evidence {
            Available {
                nested_value: Nested,
                nullable_value: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                omitted_value: Option<u32>,
            },
        }
    };
    let generated = generate_enum(&item).expect("tagged optional enum should generate");
    assert!(generated.body.contains("state_kind: \"available\";"));
    assert!(generated.body.contains("nested_value: Nested;"));
    assert!(generated.body.contains("nullable_value: string | null;"));
    assert!(generated.body.contains("omitted_value?: number;"));
    assert!(generated.dependencies.contains("Nested"));
}

#[test]
fn ambiguous_or_unsupported_serde_names_fail_closed() {
    let directional: syn::ItemStruct = syn::parse_quote! {
        #[derive(Serialize)]
        pub struct Directional {
            #[serde(rename(serialize = "sent", deserialize = "received"))]
            pub value: String,
        }
    };
    assert!(
        generate_struct(&directional)
            .err()
            .expect("directional renames need a single browser wire name")
            .to_string()
            .contains("directional")
    );

    let duplicate: syn::ItemStruct = syn::parse_quote! {
        #[derive(Serialize)]
        pub struct Duplicate {
            #[serde(rename = "first", rename = "second")]
            pub value: String,
        }
    };
    assert!(
        generate_struct(&duplicate)
            .err()
            .expect("duplicate renames are ambiguous")
            .to_string()
            .contains("multiple")
    );

    let alias: syn::ItemEnum = syn::parse_quote! {
        #[derive(Serialize)]
        pub enum Alias {
            #[serde(alias = "old_value")]
            Value,
        }
    };
    assert!(
        generate_enum(&alias)
            .err()
            .expect("aliases are not one browser wire name")
            .to_string()
            .contains("alias")
    );

    let unsupported: syn::ItemStruct = syn::parse_quote! {
        #[derive(Serialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub struct Unsupported { pub value: String }
    };
    assert!(
        generate_struct(&unsupported)
            .err()
            .expect("unsupported conversion rules must be explicit")
            .to_string()
            .contains("unsupported")
    );

    let source_dir = temporary_output_dir("ambiguous-serde-source");
    let out_dir = temporary_output_dir("ambiguous-serde-output");
    fs::create_dir_all(&source_dir).expect("temporary source directory should be created");
    fs::create_dir_all(&out_dir).expect("temporary output directory should be created");
    fs::write(
        source_dir.join("directional.rs"),
        "#[derive(Serialize)]\npub struct Directional { #[serde(rename(serialize = \"sent\"))] pub value: String }\n",
    )
    .expect("ambiguous source fixture should be written");
    let stale = out_dir.join("Stale.ts");
    fs::write(
        &stale,
        format!("{GENERATED_HEADER}\nexport type Stale = string;\n"),
    )
    .expect("owned output sentinel should be written");
    let authored = out_dir.join("Authored.ts");
    fs::write(&authored, "export type Authored = string;\n")
        .expect("authored output sentinel should be written");
    assert!(run(&[source_dir.as_path()], &out_dir).is_err());
    assert!(
        stale.exists(),
        "metadata rejection must happen before cleanup"
    );
    assert!(
        authored.exists(),
        "metadata rejection must preserve authored output before cleanup"
    );
    fs::remove_dir_all(source_dir).expect("temporary source should be removed");
    fs::remove_dir_all(out_dir).expect("temporary output should be removed");
}
