//! Generator contracts required by reusable-curriculum browser types.

use super::model::{generate_enum, generate_struct};

#[test]
fn boxed_values_generate_as_their_wire_payload() {
    let item: syn::ItemStruct = syn::parse_quote! {
        #[derive(Serialize)]
        pub struct Holder {
            pub value: Box<ReusableQuestionView>,
        }
    };
    let generated = generate_struct(&item).expect("generation should erase Box ownership");
    assert!(generated.body.contains("value: ReusableQuestionView;"));
    assert!(generated.dependencies.contains("ReusableQuestionView"));
    assert!(!generated.dependencies.contains("Box"));
}

#[test]
fn an_internally_tagged_newtype_variant_intersects_its_payload() {
    let item: syn::ItemEnum = syn::parse_quote! {
        #[derive(Serialize)]
        #[serde(tag = "kind", rename_all = "camelCase")]
        pub enum Entry {
            Fixed(FixedInput),
            Pool(PoolInput),
        }
    };
    let generated = generate_enum(&item).expect("tagged newtype payload should generate");
    assert!(generated.body.contains("{ kind: \"fixed\" }"));
    assert!(generated.body.contains("& FixedInput"));
    assert!(generated.body.contains("{ kind: \"pool\" }"));
    assert!(generated.body.contains("& PoolInput"));
    assert!(generated.dependencies.contains("FixedInput"));
    assert!(generated.dependencies.contains("PoolInput"));
}
