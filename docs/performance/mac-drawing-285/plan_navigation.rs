// Reproduce in crates/ui-egui/examples on v0.11.0 and on main.
use bimcraft_core::{engine::Session, geom::P2, model::Element};
use bimcraft_ui_egui::{BimApp, Sheet};
use egui::{Event, Modifiers, MouseWheelUnit, PointerButton, TouchPhase, pos2, vec2};
use egui_kittest::Harness;
use serde_json::json;
use std::{
    env,
    hint::black_box,
    thread::sleep,
    time::{Duration, Instant},
};

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    let path = args.get(2).ok_or("expected create|measure fixture.bimc")?;
    let mut session = Session::new();
    if args[1] == "create" {
        session.execute("highrise", &json!({"storeys":20}))?;
        for x in 0..8 {
            for y in 0..6 {
                session.execute(
                    "detail_line",
                    &json!({"centre":[x*6000+3000,y*6000+3000],"radius":1200}),
                )?;
            }
        }
        session.execute(
            "section",
            &json!({"start":[-1000,18000],"end":[49000,18000]}),
        )?;
        session.execute("elevation", &json!({"facing":"south"}))?;
        session.execute("save", &json!({"path":path}))?;
        return Ok(());
    }
    session.execute("open", &json!({"path":path}))?;
    let count = session.model.elements.len();
    let mut builder = Harness::builder().with_size([1280.0, 800.0]);
    let view = args.get(3).map(String::as_str).unwrap_or("plan");
    let mut app = BimApp::new(session);
    if view != "plan" {
        let id = app
            .session
            .model
            .elements
            .iter()
            .find_map(|(id, e)| {
                matches!(
                    (view, e),
                    ("section", Element::Section(_)) | ("elevation", Element::Elevation(_))
                )
                .then_some(*id)
            })
            .ok_or("view missing")?;
        app.sheet = Sheet::Section(id);
    }
    let gpu = args[1] == "metal";
    if gpu {
        use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
        let state = create_render_state(
            default_wgpu_setup(),
            eframe::egui_wgpu::RendererOptions::default(),
        );
        eprintln!("Offscreen renderer: {:?}", state.adapter.get_info().backend);
        assert_eq!(
            state.adapter.get_info().backend,
            eframe::egui_wgpu::wgpu::Backend::Metal
        );
        app.use_graphics_card(&state);
        builder = builder.renderer(WgpuTestRenderer::from_render_state(state));
    }
    let mut h = builder.build_ui_state(|ui, app: &mut BimApp| app.show(ui), app);
    assert!(matches!(
        (view, h.state().sheet),
        ("plan", Sheet::Plan(_)) | ("section" | "elevation", Sheet::Section(_))
    ));
    let location = |h: &Harness<BimApp>| h.state().screen(P2(0., 0.));
    let scale = |h: &Harness<BimApp>| h.state().screen(P2(10000., 0.)).distance(location(h));
    let centre = pos2(620., 420.);
    h.hover_at(centre);
    // Allow asynchronous drawing preparation and font caches to settle.
    for _ in 0..90 {
        h.step();
        if gpu {
            black_box(h.render()?);
        }
        sleep(Duration::from_millis(15));
    }
    if let Some(path) = args.get(4) {
        h.render()?.save(path).map_err(|e| e.to_string())?;
    }
    let mut report = json!({"elements":count,"view":view,"sheet":format!("{:?}",h.state().sheet),"mode":args[1]});
    for mode in ["pan", "zoom"] {
        if mode == "pan" {
            h.event(Event::PointerButton {
                pos: centre,
                button: PointerButton::Middle,
                pressed: true,
                modifiers: Modifiers::NONE,
            });
            h.step();
        }
        let mut samples = Vec::new();
        let start_pt = location(&h);
        let start_scale = scale(&h);
        let mut max_move = 0.0f32;
        let mut max_scale = 0.0f32;
        for i in 0..120 {
            let phase = i % 20;
            let offset = if phase < 10 { phase + 1 } else { 19 - phase };
            if mode == "pan" {
                h.hover_at(centre + vec2(offset as f32 * 4., offset as f32 * 2.));
            } else {
                h.event(Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: vec2(0., if phase < 10 { 6. } else { -6. }),
                    phase: TouchPhase::Move,
                    modifiers: Modifiers::NONE,
                });
            }
            let start = Instant::now();
            h.step();
            if gpu {
                black_box(h.render()?);
            } else {
                let output = h.output();
                let prims = h
                    .ctx
                    .tessellate(output.shapes.clone(), output.pixels_per_point);
                black_box(prims);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.;
            if i >= 20 {
                samples.push(ms);
            }
            max_move = max_move.max(location(&h).distance(start_pt));
            max_scale = max_scale.max((scale(&h) / start_scale - 1.).abs());
            sleep(Duration::from_millis(10));
        }
        if mode == "pan" {
            h.event(Event::PointerButton {
                pos: centre,
                button: PointerButton::Middle,
                pressed: false,
                modifiers: Modifiers::NONE,
            });
            h.step();
            assert!(max_move > 20., "pan never moved the drawing: {max_move}");
        } else {
            assert!(
                max_scale > 0.10,
                "zoom never changed the drawing: {max_scale}"
            );
        }
        samples.sort_by(f64::total_cmp);
        report[mode] = json!({"samples":samples.len(),"median_ms":(samples[49]+samples[50])/2.,"p95_ms":samples[94],"max_ms":samples[99],"max_displacement_pt":max_move,"max_relative_scale_change":max_scale});
    }
    println!("{report}");
    Ok(())
}
