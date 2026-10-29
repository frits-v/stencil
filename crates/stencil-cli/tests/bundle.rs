#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! `render --themes` (section 5.2): one SVG that carries several themes. Each bundled
//! theme, resolved from its own variable block, draws what its single render draws.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use stencil_cli::pipeline::{
    LoadedDocument, RenderedPage, SvgForm, ThemeChoice, bundled_svg, load_bundle_themes,
    load_document_from, read_input, render_page,
};
use stencil_cli::{ExitCode, run};
use stencil_model::{BUILTIN_THEMES, DESIGNED_THEMES};
use stencil_render::{DeviceScale, render_png};

const DEFAULT_THEME: &str = "center";

struct Outcome {
    code: ExitCode,
    stdout: String,
    stderr: String,
}

fn run_stencil(arguments: &[&str]) -> Outcome {
    let mut argv: Vec<OsString> = vec!["stencil".into()];
    argv.extend(arguments.iter().map(OsString::from));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run(argv, &mut stdout, &mut stderr);
    Outcome {
        code,
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn scratch_directory(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli")
        .join(name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    directory
}

/// The example loaded with `theme` as `--theme` sets it, and rendered.
fn render_example_under(stem: &str, theme: &str) -> (LoadedDocument, RenderedPage) {
    let input = repository_path(&format!("examples/{stem}.json"));
    let json_text = read_input(&input).unwrap();
    let choice = ThemeChoice {
        reference: theme,
        directory: Path::new("."),
    };
    let loaded = load_document_from(&json_text, input.parent().unwrap(), Some(choice)).unwrap();
    let rendered = render_page(&loaded, DeviceScale::DEFAULT).unwrap();
    (loaded, rendered)
}

/// The single-theme written file of the example under `theme`, in `form`, with the
/// PNG of the file and the number of its text elements.
fn single_render(stem: &str, theme: &str, form: SvgForm) -> (String, Vec<u8>, usize) {
    let (_, rendered) = render_example_under(stem, theme);
    let svg = match form {
        SvgForm::SelfContained => rendered.svg.self_contained().unwrap(),
        SvgForm::Portable => rendered.svg.portable().unwrap(),
    };
    (svg, rendered.png, rendered.svg.text_elements)
}

/// The bundle of the example over `themes`, the first the default, as `render --themes`
/// writes it, with the pipeline's PNG and the default's text element count.
fn bundle(stem: &str, themes: &[&str], form: SvgForm) -> (String, Vec<u8>, usize) {
    let (loaded, rendered) = render_example_under(stem, themes[0]);
    let further: Vec<String> = themes[1..].iter().map(|name| name.to_string()).collect();
    let further_themes = load_bundle_themes(&loaded.page, &further).unwrap();
    let svg = bundled_svg(&loaded, &rendered, &further_themes, form).unwrap();
    (svg, rendered.png, rendered.svg.text_elements)
}

/// The variables of the bundle's block for `theme`, name to value.
fn theme_variables(bundle_svg: &str, theme: &str) -> BTreeMap<String, String> {
    let selector = format!(r#"svg[data-themes][data-theme="{theme}"] {{"#);
    let line = bundle_svg
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&selector))
        .unwrap_or_else(|| panic!("no variable block for {theme}"));
    let declarations = line
        .strip_prefix(&selector)
        .unwrap()
        .trim_end()
        .strip_suffix('}')
        .unwrap();
    declarations
        .split(';')
        .map(str::trim)
        .filter(|declaration| !declaration.is_empty())
        .map(|declaration| {
            let (name, value) = declaration.split_once(':').unwrap();
            (name.trim().to_string(), value.trim().to_string())
        })
        .collect()
}

/// The bundle as a single-theme file of `theme`: every variable replaced by that theme's
/// value, the lines the theme hides dropped, the bundle's own style block and the root's
/// `data-themes` removed.
fn resolve(bundle_svg: &str, theme: &str) -> String {
    let variables = theme_variables(bundle_svg, theme);
    let mut lines = bundle_svg.lines();
    let root = lines.next().unwrap();
    let mut output = String::new();
    output.push_str(&root.replace(&format!(r#" data-themes="{}""#, root_themes(root)), ""));
    output.push('\n');
    assert_eq!(lines.next().unwrap(), "  <defs>");
    assert_eq!(lines.next().unwrap(), "    <style>");
    for line in lines.by_ref() {
        if line == "    </style>" {
            break;
        }
    }
    assert_eq!(lines.next().unwrap(), "  </defs>");
    for line in lines {
        let mut resolved = line.to_string();
        for (name, value) in &variables {
            resolved = resolved.replace(&format!("var({name})"), value);
        }
        assert!(
            !resolved.contains("var(--"),
            "unresolved variable: {resolved}"
        );
        if resolved.contains(r#" display="none""#) {
            continue;
        }
        output.push_str(&resolved.replace(r#" display="inline""#, ""));
        output.push('\n');
    }
    output
}

fn root_themes(root: &str) -> String {
    let (_, rest) = root.split_once(r#" data-themes=""#).unwrap();
    rest.split_once('"').unwrap().0.to_string()
}

fn png_of(svg: &str, text_elements: usize) -> Vec<u8> {
    render_png(svg, text_elements, DeviceScale::new(1).unwrap()).unwrap()
}

fn variable_uses(svg: &str) -> usize {
    svg.matches("var(--").count()
}

#[test]
fn every_builtin_theme_resolves_from_a_bundle_to_its_own_render() {
    for form in [SvgForm::SelfContained, SvgForm::Portable] {
        let mut themes_examined = 0;
        let mut uses = 0;
        let (_, default_png, _) = single_render("g7", DEFAULT_THEME, form);
        for theme in BUILTIN_THEMES {
            let pair = if theme == DEFAULT_THEME {
                [DEFAULT_THEME, "clear-dark"]
            } else {
                [DEFAULT_THEME, theme]
            };
            let (bundle_svg, bundle_png, text_elements) = bundle("g7", &pair, form);
            assert_eq!(
                bundle_png, default_png,
                "{theme} {form:?}: the PNG is the default's"
            );
            let (single_svg, _, single_texts) = single_render("g7", theme, form);
            assert_eq!(single_texts, text_elements, "{theme} {form:?}");
            let resolved = resolve(&bundle_svg, theme);
            assert_eq!(
                png_of(&resolved, text_elements),
                png_of(&single_svg, text_elements),
                "{theme} {form:?}: the resolved bundle draws differently from the single render"
            );
            uses += variable_uses(&bundle_svg);
            themes_examined += 1;
        }
        assert_eq!(themes_examined, 13, "{form:?}");
        assert!(
            uses > 50 * 13,
            "{form:?}: {uses} variable uses over 13 bundles"
        );
    }
}

#[test]
fn the_designed_themes_bundle_on_an_iso_figure() {
    let (bundle_svg, _, text_elements) =
        bundle("hero-iso", &DESIGNED_THEMES, SvgForm::SelfContained);
    let mut themes_examined = 0;
    for theme in DESIGNED_THEMES {
        let (single_svg, _, single_texts) =
            single_render("hero-iso", theme, SvgForm::SelfContained);
        assert_eq!(single_texts, text_elements, "{theme}");
        let resolved = resolve(&bundle_svg, theme);
        assert_eq!(
            png_of(&resolved, text_elements),
            png_of(&single_svg, text_elements),
            "{theme}: the resolved bundle draws differently from the single render"
        );
        themes_examined += 1;
    }
    assert_eq!(themes_examined, DESIGNED_THEMES.len());
    // dusk and clear-dark blur the block shadow by 3 where center blurs by 2.5, so their
    // filters are kept under their own ids and the blocks pick theirs by variable.
    for renamed in ["stencil-shadow-dusk", "stencil-shadow-clear-dark"] {
        assert!(
            bundle_svg.contains(&format!(r#"<filter id="{renamed}""#)),
            "{renamed}"
        );
    }
    assert!(bundle_svg.contains(r#"filter="var(--stencil-filter-"#));
}

/// The golden bundle: g7 over center, clear-dark and wire, without the `@font-face` lines,
/// which hold the font bytes.
fn golden_bundle() -> String {
    let (bundle_svg, _, _) = bundle(
        "g7",
        &["center", "clear-dark", "wire"],
        SvgForm::SelfContained,
    );
    let mut kept = String::new();
    for line in bundle_svg.lines() {
        if !line.contains("@font-face") {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    kept
}

#[test]
fn a_theme_bundle_matches_its_golden() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/g7.themes.svg");
    let fixture = fs::read_to_string(&fixture_path).unwrap();
    assert!(fixture.contains("@media (prefers-color-scheme: dark)"));
    assert!(fixture.contains(r#"display="var("#));
    let bundle_svg = golden_bundle();
    if bundle_svg != fixture {
        let differing = bundle_svg
            .lines()
            .zip(fixture.lines())
            .position(|(drawn, expected)| drawn != expected)
            .map_or(
                bundle_svg.lines().count().min(fixture.lines().count()),
                |index| index,
            );
        panic!(
            "the g7 bundle differs from tests/fixtures/g7.themes.svg at line {}:\n  drawn:    {}\n  expected: {}\nregenerate the fixture when the change is intended",
            differing + 1,
            bundle_svg.lines().nth(differing).unwrap_or(""),
            fixture.lines().nth(differing).unwrap_or("")
        );
    }
}

#[test]
fn a_theme_that_lays_out_differently_is_refused() {
    let directory = scratch_directory("bundle_refused");
    fs::create_dir_all(&directory).unwrap();
    let mut theme: serde_json::Value =
        serde_json::from_str(stencil_render::builtin_theme_json("center").unwrap()).unwrap();
    theme["name"] = serde_json::json!("thick");
    theme["iso"]["slab_thickness"] = serde_json::json!(8.0);
    let theme_path = directory.join("thick.json");
    fs::write(&theme_path, serde_json::to_string_pretty(&theme).unwrap()).unwrap();
    let out_dir = directory.join("out");
    let input = repository_path("examples/hero-iso.json");
    let outcome = run_stencil(&[
        "render",
        input.to_str().unwrap(),
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--themes",
        &format!("center,{}", theme_path.display()),
    ]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun, "{}", outcome.stderr);
    assert!(outcome.stdout.is_empty(), "{}", outcome.stdout);
    assert!(
        outcome.stderr.contains("theme thick cannot be bundled"),
        "{}",
        outcome.stderr
    );
    assert!(
        outcome.stderr.contains(
            "its iso slab thickness is 8 where center's is 6, so the projected canvas differs"
        ),
        "{}",
        outcome.stderr
    );
    assert!(!out_dir.exists() || fs::read_dir(&out_dir).unwrap().next().is_none());
}

#[test]
fn render_with_themes_writes_the_bundle_and_checks_each_theme() {
    let input = repository_path("examples/g7.json");
    let input = input.to_str().unwrap();

    // center fails one contrast row of its own (ask.ink on ask.fill); the row is printed
    // and render keeps its exit code, since it never fails on contrast.
    let out_dir = scratch_directory("bundle_clear_dark");
    let outcome = run_stencil(&[
        "render",
        input,
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--themes",
        "center,clear-dark",
        "--portable",
    ]);
    assert_eq!(
        outcome.code,
        ExitCode::Clean,
        "{}{}",
        outcome.stdout,
        outcome.stderr
    );
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(lines.len(), 7, "{}", outcome.stdout);
    for (line, suffix) in
        lines[..4]
            .iter()
            .zip([".svg", ".png", ".measured.json", ".description.md"])
    {
        assert!(line.ends_with(&format!("g7{suffix}")), "{line}");
    }
    assert_eq!(
        lines[4],
        "row contrast ask.ink on ask.fill: 4.34, at least 4.50, FAILED"
    );
    assert_eq!(
        lines[5],
        "stencil theme check center: 285 rows, 284 passed, 1 failed"
    );
    assert_eq!(
        lines[6],
        "stencil theme check clear-dark: 285 rows, 285 passed, 0 failed"
    );
    let svg = fs::read_to_string(out_dir.join("g7.svg")).unwrap();
    let root = svg.lines().next().unwrap();
    assert!(root.starts_with("<svg "), "{root}");
    assert!(
        root.ends_with(r#" data-themes="center clear-dark">"#),
        "{root}"
    );
    assert!(!svg.contains("<image"));
    assert!(!svg.contains("@font-face"));
    assert!(
        svg.contains("@media (prefers-color-scheme: dark) { svg[data-themes]:not([data-theme]) {")
    );
    assert!(svg.contains(r#"svg[data-themes][data-theme="clear-dark"] {"#));

    let out_dir = scratch_directory("bundle_nord");
    let outcome = run_stencil(&[
        "render",
        input,
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--themes",
        "center,nord",
    ]);
    assert_eq!(
        outcome.code,
        ExitCode::Clean,
        "{}{}",
        outcome.stdout,
        outcome.stderr
    );
    assert!(out_dir.join("g7.svg").exists());
    assert!(out_dir.join("g7.png").exists());
    let failed_rows = outcome
        .stdout
        .lines()
        .filter(|line| line.starts_with("row ") && line.ends_with("FAILED"))
        .count();
    assert!(failed_rows > 1, "{}", outcome.stdout);
    assert!(
        outcome
            .stdout
            .lines()
            .last()
            .unwrap()
            .starts_with("stencil theme check nord: 285 rows,"),
        "{}",
        outcome.stdout
    );

    let out_dir = scratch_directory("bundle_one");
    let outcome = run_stencil(&[
        "render",
        input,
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--themes",
        "center",
    ]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(
        outcome.stderr,
        "stencil render: --themes needs at least two themes\n"
    );
    assert!(outcome.stdout.is_empty());
    assert!(!out_dir.exists());

    let outcome = run_stencil(&[
        "render",
        input,
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--theme",
        "dusk",
        "--themes",
        "center,dusk",
    ]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert!(
        outcome
            .stderr
            .contains("'--theme <THEME>' cannot be used with '--themes <THEMES>'"),
        "{}",
        outcome.stderr
    );
    assert!(!out_dir.exists());
}

#[test]
fn a_default_theme_without_a_definition_keeps_every_other_theme_apart() {
    let themes = ["wire", "center", "dusk"];
    let (bundle_svg, _, text_elements) = bundle("hero-iso", &themes, SvgForm::SelfContained);
    assert!(
        bundle_svg.contains(r#"<filter id="stencil-shadow" "#),
        "center keeps the bare id"
    );
    assert!(
        bundle_svg.contains(r#"<filter id="stencil-shadow-dusk" "#),
        "dusk is renamed against center"
    );
    assert!(!bundle_svg.contains(r#"<filter id="stencil-shadow-center" "#));
    for theme in themes {
        let (single_svg, _, single_texts) =
            single_render("hero-iso", theme, SvgForm::SelfContained);
        assert_eq!(single_texts, text_elements, "{theme}");
        let resolved = resolve(&bundle_svg, theme);
        assert_eq!(
            png_of(&resolved, text_elements),
            png_of(&single_svg, text_elements),
            "{theme}: the resolved bundle draws differently from the single render"
        );
    }
}

/// Every ` id="…"` value a file carries, in order.
fn ids_of(svg: &str) -> Vec<&str> {
    let mut ids = Vec::new();
    let mut rest = svg;
    while let Some((_, after)) = rest.split_once(" id=\"") {
        let (id, following) = after.split_once('"').unwrap();
        ids.push(id);
        rest = following;
    }
    ids
}

fn example_stems() -> Vec<String> {
    let mut stems: Vec<String> = fs::read_dir(repository_path("examples"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    stems.sort();
    stems
}

/// Whether the example's projection is iso.
fn is_iso(stem: &str) -> bool {
    let (loaded, _) = render_example_under(stem, DEFAULT_THEME);
    loaded.page.projection == stencil_model::Projection::Iso
}

#[test]
fn no_example_bundle_repeats_an_id() {
    let thick_themes = ["tokyo-night", "dracula"];
    let stems = example_stems();
    assert!(stems.len() >= 10, "{} examples", stems.len());
    let mut accepted = 0;
    let mut refused = Vec::new();
    for stem in &stems {
        let iso = is_iso(stem);
        let (loaded, rendered) = render_example_under(stem, DEFAULT_THEME);
        for form in [SvgForm::SelfContained, SvgForm::Portable] {
            let further: Vec<String> = BUILTIN_THEMES
                .iter()
                .filter(|name| **name != DEFAULT_THEME)
                .filter(|name| !(iso && thick_themes.contains(name)))
                .map(|name| name.to_string())
                .collect();
            let further_themes = load_bundle_themes(&loaded.page, &further).unwrap();
            let bundle_svg = bundled_svg(&loaded, &rendered, &further_themes, form).unwrap();
            let ids = ids_of(&bundle_svg);
            let distinct: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
            assert_eq!(ids.len(), distinct.len(), "{stem} {form:?} repeats an id");
            accepted += 1;
            if iso {
                for name in thick_themes {
                    let further_themes =
                        load_bundle_themes(&loaded.page, &[name.to_string()]).unwrap();
                    let error = bundled_svg(&loaded, &rendered, &further_themes, form).unwrap_err();
                    let message = error.to_string();
                    assert!(
                        message.contains(&format!(
                            "theme {name} cannot be bundled: its iso slab thickness is 8 where center's is 6"
                        )),
                        "{stem} {form:?}: {message}"
                    );
                    refused.push(format!("{stem} {name}"));
                }
            }
        }
    }
    assert_eq!(accepted, stems.len() * 2);
    let iso_count = stems.iter().filter(|stem| is_iso(stem)).count();
    assert!(iso_count >= 5, "{iso_count} iso examples");
    assert_eq!(refused.len(), iso_count * 2 * 2);
}
