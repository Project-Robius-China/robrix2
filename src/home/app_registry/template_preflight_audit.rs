use super::splash_host::DefaultSplashHost;
use super::templates::ALL_TEMPLATES;

#[test]
fn all_templates_pass_preflight_at_build_time() {
    // Iterate the canonical `ALL_TEMPLATES` table (single source of
    // truth per P1a). If someone adds a new template const without
    // registering it here, the preflight audit will miss it —
    // `all_templates_sources_match_individual_consts` in
    // `templates.rs` catches that drift separately.
    let host = DefaultSplashHost::new();
    for (capability_id, template_id, source) in ALL_TEMPLATES {
        host.validate_source_for_test(capability_id, template_id, source)
            .unwrap_or_else(|err| {
                panic!(
                    "template preflight failed for {capability_id}/{template_id}: {err}"
                )
            });
    }
}

#[test]
fn no_provenance_storage_new_modules() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/home/app_registry");
    for forbidden in ["render_receipt.rs", "provenance.rs", "audit.rs"] {
        assert!(
            !base.join(forbidden).exists(),
            "template-runtime forbids provenance/receipt module {forbidden}"
        );
    }
}
