mod canvas;
mod color;
mod sphere;
mod vec3;

use canvas::Canvas;
use color::Color;
use sphere::Sphere;
use vec3::Vec3;

use rand::random;
use rayon::prelude::*;
use std::path::Path;
use std::time::Instant;

fn objects() -> Vec<Sphere> {
    vec![
        // Light
        Sphere {
            radius: 0.2,
            center: Vec3 {
                x: 0.,
                y: 1.25,
                z: -0.5,
            },
            color: Color {
                r: 1.,
                g: 1.,
                b: 1.,
            },
            diffuseness: 1.0,
        },
        // Floor
        Sphere {
            radius: 20000.25,
            center: Vec3 {
                x: 0.5,
                y: -20000.,
                z: 0.5,
            },
            color: Color {
                r: 234. / 255.,
                g: 21. / 255.,
                b: 81. / 255.,
            },
            diffuseness: 1.0,
        },
        // Blue sphere in middle
        Sphere {
            radius: 0.25,
            center: Vec3 {
                x: 0.5,
                y: 0.5,
                z: 0.5,
            },
            color: Color {
                r: 0.,
                g: 0.,
                b: 1.,
            },
            diffuseness: 1.0,
        },
        // Green sphere on right
        Sphere {
            radius: 0.25,
            center: Vec3 {
                x: 1.0,
                y: 0.5,
                z: 1.0,
            },
            color: Color {
                r: 0.,
                g: 1.,
                b: 0.,
            },
            diffuseness: 0.5,
        },
        // Mirror sphere on left
        Sphere {
            radius: 0.5,
            center: Vec3 {
                x: 0.0,
                y: 0.75,
                z: 1.25,
            },
            color: Color {
                r: 0.999,
                g: 0.999,
                b: 0.999,
            },
            diffuseness: 0.01,
        },
    ]
}

pub fn cos_dir(normal: Vec3) -> Vec3 {
    let nor_xy_sq = Vec3::new(normal.x * normal.x, normal.y * normal.y, 0.);

    let mut tc = Vec3::new(
        1.0 + normal.z - nor_xy_sq.x,
        1.0 + normal.z - nor_xy_sq.y,
        -normal.x * normal.y,
    );

    tc *= 1. / (1. + normal.z);

    let uu = Vec3::new(tc.x, tc.z, -normal.x);
    let vv = Vec3::new(tc.z, tc.y, -normal.y);

    let u = random::<f64>();
    let v = random::<f64>();

    let a = std::f64::consts::TAU * v;

    (uu * a.cos() + vv * a.sin()) * u.sqrt() + normal * (1.0 - u).sqrt()
}

pub fn get_color(
    origin: Vec3,
    dir: Vec3,
    light: Vec3,
    spheres: &[Sphere],
    depth: i8,
    max_depth: i8,
) -> Option<Color> {
    if depth == max_depth {
        return None;
    }

    let hit = spheres
        .iter()
        .filter_map(|x| x.intersect(origin, dir))
        .min_by(|(_, _, t1), (_, _, t2)| t1.partial_cmp(t2).unwrap());

    let (sphere, intersection, _) = match hit {
        None => {
            return None;
        }
        Some(tup) => tup,
    };

    // If we hit the light, return.
    if (sphere.color.r - 1.).abs() < 0.001
        && (sphere.color.g - 1.).abs() < 0.001
        && (sphere.color.b - 1.).abs() < 0.001
    {
        return Some(sphere.color);
    }

    let gradient = (intersection - sphere.center).as_normal();

    return if random::<f64>() > sphere.diffuseness {
        // Ideal specular reflection
        let specular_dir = dir - gradient * 2. * gradient.dot(dir);

        Some(
            sphere.color
                * get_color(
                    intersection,
                    specular_dir,
                    light,
                    spheres,
                    depth + 1,
                    max_depth,
                )
                .unwrap_or_else(Color::black),
        )
    } else {
        let light_dir = intersection - light;

        let rand_light_dir = cos_dir(light_dir * -1.);

        // 0.2 is light radius.
        let cos_a_max = (1. - (0.2 * 0.2) / light_dir.dot(light_dir)).sqrt();

        let (eps1, eps2) = (random::<f64>(), random::<f64>());
        let cos_a = 1. - eps1 + eps1 * cos_a_max;
        let sin_a = (1. - cos_a * cos_a).sqrt();
        let phi = 2. * std::f64::consts::PI * eps2;

        let rand_light_dir = (rand_light_dir * phi.cos() * sin_a
            + rand_light_dir.cross(light_dir) * phi.sin() * sin_a
            + light_dir * cos_a)
            .as_normal();

        let shadow = spheres
            .iter()
            .skip(1)
            .any(|x| x.intersect(intersection, rand_light_dir).is_some());

        let explicit = if shadow {
            let omega = 2. * std::f64::consts::PI * (1. - cos_a_max);

            sphere.color * rand_light_dir.dot(gradient) * omega * std::f64::consts::FRAC_1_PI
        } else {
            Color::black()
        };

        let cos_dir = cos_dir(gradient);

        // Random diffuse in the hemisphere, cos-weighted distribution
        Some(
            explicit
                + sphere.color
                    * get_color(intersection, cos_dir, light, spheres, depth + 1, max_depth)
                        .unwrap_or_else(Color::white),
        )
    };
}

fn main() {
    let max_y = 1080;
    let max_x = 1920;

    let eye = Vec3 {
        x: 0.5,
        y: 0.5,
        z: -1.,
    };

    let light = Vec3 {
        x: 0.,
        y: 1.25,
        z: -0.5,
    };

    let spheres = objects();

    let time = Instant::now();

    let mut canvas = Canvas::new(max_x, max_y);

    // Anti-aliasing iterations.
    const ITERS: u64 = 256;

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let mut c = (0..ITERS)
                .into_par_iter()
                .filter_map(|_| {
                    let px_scaled = -0.25 + (x as f64 + random::<f64>()) / max_y as f64;
                    let py_scaled = ((max_y - y) as f64 + random::<f64>()) / max_y as f64;

                    let mut origin = eye;

                    let mut ray_dir = eye.ray_to(Vec3::new(px_scaled, py_scaled, 0.));

                    // blur * vec3
                    let go = (Vec3::new(rand::random::<f64>(), random::<f64>(), 0.) * 2.0 - 1.0)
                        * 0.0015;

                    let gd = ray_dir * 0.6 - go;

                    origin.x += go.x;
                    origin.y += go.y;

                    ray_dir.x += gd.x;
                    ray_dir.y += gd.y;

                    ray_dir.normalize();

                    get_color(eye, ray_dir, light, &spheres, 0, 20)
                })
                .sum::<Color>();

            c *= 1. / ITERS as f64;

            canvas.set(x, y, c);
        }
    }

    canvas
        .write(Path::new("out.ppm"))
        .expect("Unable to write to file.");

    println!("{}", time.elapsed().as_secs_f32());
}
