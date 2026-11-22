use i18n_embed::{FileSystemAssets, LanguageLoader, fluent::FluentLanguageLoader};

fn main() {
    let loader = FluentLanguageLoader::new("generated-macro", "en-US".parse().unwrap());
    let assets = FileSystemAssets::try_new(concat!(env!("CARGO_MANIFEST_DIR"), "/i18n")).unwrap();
    loader
        .load_languages(&assets, &["en-US".parse().unwrap()])
        .unwrap();
    let plain: String = i18n_embed_fl::fl!(&loader, "plain");
    assert_eq!(plain, "Generated hello");
    let named: String = i18n_embed_fl::fl!(&loader, "greeting", name = "Ada", place = "Dubai");
    assert!(named.contains("Ada") && named.contains("Dubai"));
    let attribute: String = i18n_embed_fl::fl!(&loader, "greeting", "tooltip", name = "Ada");
    assert!(attribute.contains("Generated tooltip") && attribute.contains("Ada"));
    println!("OXBOX_MACRO_VALID_PASS");
}
