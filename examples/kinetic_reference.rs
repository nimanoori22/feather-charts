use feather_charts::model::{
    invalidate_mask::TimeScaleAnimation, kinetic_animation::KineticSampler,
};
use std::time::{Duration, Instant};
fn main() {
    let gestures: &[&[(u64, f64)]] = &[
        &[(0, 0.), (10, 10.), (20, 20.), (30, 40.)],
        &[(0, 0.), (10, -10.), (20, -25.), (30, -40.)],
        &[(0, 0.), (10, 100.), (20, 200.), (30, 150.)],
        &[(0, 0.), (10, 0.1), (20, 0.2), (30, 0.3)],
        &[(0, 0.), (10, 10.), (10, 12.), (30, 40.)],
        &[(0, 0.), (10, 10.)],
        &[(0, 0.)],
    ];
    let mut cases = vec![];
    for spacing in [0.5, 1., 6., 12., 20.] {
        for samples in gestures {
            for delay in [0, 20, 50, 51, 100] {
                let origin = Instant::now();
                let mut sampler =
                    KineticSampler::new(0.2 / spacing, 7. / spacing, 0.997, 15. / spacing).unwrap();
                for &(ms, position) in *samples {
                    sampler.add_position(position, origin + Duration::from_millis(ms));
                }
                let &(last_ms, last_position) = samples.last().unwrap();
                let start_ms = last_ms + delay;
                let animation =
                    sampler.start(last_position, origin + Duration::from_millis(start_ms));
                let result = animation.map_or("null".into(), |animation| {
                    let results = [0, 1, 16, 100, 1_000, 10_000].map(|elapsed| {
                        let (position, finished) =
                            animation.sample(origin + Duration::from_millis(start_ms + elapsed));
                        format!("[{elapsed},{position},{finished}]")
                    });
                    format!("[{}]", results.join(","))
                });
                let sample_json = samples
                    .iter()
                    .map(|(ms, position)| format!("[{ms},{position}]"))
                    .collect::<Vec<_>>()
                    .join(",");
                cases.push(format!("{{\"spacing\":{spacing},\"samples\":[{sample_json}],\"start\":{start_ms},\"position\":{last_position},\"result\":{result}}}"));
            }
        }
    }
    println!("[{}]", cases.join(","));
}
