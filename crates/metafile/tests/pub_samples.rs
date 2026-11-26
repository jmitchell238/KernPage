//! Parse every WMF/EMF blip of the (git-ignored, untrusted) POI sample files with our own
//! code only. Skips gracefully when the samples are absent.

use pub_format::PictureFormat;

fn sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(kernpage_samples::find(&format!("external/poi/{name}"))?).ok()
}

fn run(name: &str) {
    let Some(bytes) = sample(name) else {
        eprintln!("{name}: sample absent, skipping");
        return;
    };
    let doc = pub_format::read_bytes(&bytes).expect("pub-format reads sample");
    let mut n = 0;
    for page in &doc.pages {
        for shape in &page.shapes {
            let Some(pic) = &shape.picture else { continue };
            let t = match pic.format {
                PictureFormat::Emf => 2,
                PictureFormat::Wmf => 3,
                _ => continue,
            };
            n += 1;
            let raw = &pic.bytes;
            match metafile::parse(raw) {
                Ok(mf) => {
                    let kinds =
                        |f: fn(&metafile::Item) -> bool| mf.items.iter().filter(|i| f(i)).count();
                    eprintln!(
                        "{name} blip#{n} type {t}: {} bytes, records={} items={} (bitmaps={}, placeholders={}, text={}) skipped={} {:?} ignored={} truncated={} bounds={:?} inch={:?}",
                        raw.len(),
                        mf.records,
                        mf.items.len(),
                        kinds(|i| matches!(i, metafile::Item::Bitmap { .. })),
                        kinds(|i| matches!(i, metafile::Item::BitmapPlaceholder { .. })),
                        kinds(|i| matches!(i, metafile::Item::Text(_))),
                        mf.skipped,
                        mf.skipped_types,
                        mf.ignored,
                        mf.truncated,
                        mf.bounds,
                        mf.inch
                    );
                    assert!(!mf.items.is_empty(), "{name} blip#{n} produced no items");
                }
                Err(e) => panic!("{name} blip#{n}: {e}"),
            }
        }
    }
    eprintln!("{name}: {n} metafile pictures");
}

#[test]
fn brochure_blips() {
    run("SampleBrochure.pub");
}

#[test]
fn newsletter_blips() {
    run("SampleNewsletter.pub");
}
