use i18n_embed::fluent::FluentLanguageLoader;

fn main() {
    let loader = FluentLanguageLoader::new("generated-macro", "en-US".parse().unwrap());
    let _: String = i18n_embed_fl::fl!(&loader, "greeting", name = "Ada");
}
