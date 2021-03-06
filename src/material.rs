use rand::distributions::{Distribution, Uniform};
use rand::rngs::ThreadRng;

use crate::colour::Colour;
use crate::hittable::HitRecord;
use crate::ray::Ray;
use crate::vec::{reflect, refract, Vec3};

use crate::texture::Texture;

/// Schlick's approximation
fn schlick(cos: f64, ior: f64) -> f64 {
    let r0 = ((1.0 - ior) / (1.0 + ior)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos).powi(5)
}

pub trait Material: Sync {
    /// Given an input ray and a record of a collision, calculate the reflected ray and the Colour of the point.
    fn scatter(
        &self,
        rec: &HitRecord,
        ray: &Ray,
        dist: &Uniform<f64>,
        rng: &mut ThreadRng,
    ) -> Option<(Ray, Colour)>;

    /// Return how much light is emitted from the material. Black for anything that isn't a light source.
    fn emitted(&self, u: f64, v: f64, p: Vec3, dist: &Uniform<f64>, rng: &mut ThreadRng) -> Colour;
}

/// Lambertian materials a diffuse. For this program, they reflect 50% of light.
#[derive(Debug, Clone, Copy)]
pub struct Lambertian<T: Texture> {
    albedo: T,
}

impl<T: Texture> Lambertian<T> {
    pub fn new(albedo: T) -> Self {
        Self { albedo }
    }
}

impl<T: Texture> Material for Lambertian<T> {
    fn scatter(
        &self,
        rec: &HitRecord,
        ray: &Ray,
        dist: &Uniform<f64>,
        rng: &mut ThreadRng,
    ) -> Option<(Ray, Colour)> {
        let scattered_ray = Ray::new(
            rec.p,
            rec.normal + Vec3::random_in_unit_sphere(dist, rng),
            ray.time,
        );

        let attenuation = self.albedo.value(rec.u, rec.v, rec.p);
        Some((scattered_ray, attenuation))
    }

    fn emitted(
        &self,
        _u: f64,
        _v: f64,
        _p: Vec3,
        _dist: &Uniform<f64>,
        _rng: &mut ThreadRng,
    ) -> Colour {
        Colour::new(0.0, 0.0, 0.0)
    }
}

/// Metallic materials are reflective. They have a colour and a 'fuzz' value, determining how crisp the reflections are.
#[derive(Debug, Clone, Copy)]
pub struct Metal {
    albedo: Colour,
    fuzz: f64,
}

impl Metal {
    pub fn new(albedo: Colour, fuzz: f64) -> Self {
        Self { albedo, fuzz }
    }
}

impl Material for Metal {
    fn scatter(
        &self,
        rec: &HitRecord,
        ray: &Ray,
        dist: &Uniform<f64>,
        rng: &mut ThreadRng,
    ) -> Option<(Ray, Colour)> {
        let reflected_ray = reflect(ray.direction.normalise(), rec.normal);

        if reflected_ray.dot(rec.normal) > 0.0 {
            let scattered_ray = Ray::new(
                rec.p,
                reflected_ray + Vec3::random_in_unit_sphere(dist, rng) * self.fuzz,
                ray.time,
            );
            Some((scattered_ray, self.albedo))
        } else {
            None
        }
    }

    fn emitted(
        &self,
        _u: f64,
        _v: f64,
        _p: Vec3,
        _dist: &Uniform<f64>,
        _rng: &mut ThreadRng,
    ) -> Colour {
        Colour::new(0.0, 0.0, 0.0)
    }
}

/// Dielectric materials are transparent. Given an IOR they will refract light, or reflect it where applicable.
#[derive(Debug, Clone, Copy)]
pub struct Dielectric {
    ior: f64,
}

impl Dielectric {
    pub fn new(ior: f64) -> Self {
        Self { ior }
    }
}

impl Material for Dielectric {
    fn scatter(
        &self,
        rec: &HitRecord,
        ray: &Ray,
        dist: &Uniform<f64>,
        rng: &mut ThreadRng,
    ) -> Option<(Ray, Colour)> {
        let attenuation = Colour::new(1.0, 1.0, 1.0);

        let ni_over_nt = if rec.front_face {
            1.0 / self.ior
        } else {
            self.ior
        };

        let unit_direction = ray.direction.normalise();
        let cos = (-unit_direction).dot(rec.normal).min(1.0);

        // Match on whether or not refraction is possible given the ratio between ior's
        match refract(unit_direction, rec.normal, ni_over_nt) {
            // If refraction is possible, use Shclick's approximation to choose between reflection and refraction
            Some(refracted) => {
                // If Shlick's approximation says reflection is highly likely, use Uniform dist decide whether to reflect
                let reflect_prob = schlick(cos, ni_over_nt);
                if dist.sample(rng) < reflect_prob {
                    let reflected = reflect(unit_direction, rec.normal);
                    let scattered = Ray::new(rec.p, reflected, ray.time);
                    return Some((scattered, attenuation));
                }

                // Otherwise refract the ray
                let scattered = Ray::new(rec.p, refracted, ray.time);
                Some((scattered, attenuation))
            }

            // Reflect the ray if no refraction is possible
            None => {
                let reflected = reflect(unit_direction, rec.normal);
                let scattered = Ray::new(rec.p, reflected, ray.time);
                Some((scattered, attenuation))
            }
        }
    }

    fn emitted(
        &self,
        _u: f64,
        _v: f64,
        _p: Vec3,
        _dist: &Uniform<f64>,
        _rng: &mut ThreadRng,
    ) -> Colour {
        Colour::new(0.0, 0.0, 0.0)
    }
}

/// DiffuseLight materials emit light of a specified colour.
#[derive(Debug, Clone, Copy)]
pub struct DiffuseLight<T: Texture> {
    emit: T,
}

impl<T: Texture> DiffuseLight<T> {
    pub fn new(emit: T) -> Self {
        Self { emit }
    }
}

impl<T: Texture> Material for DiffuseLight<T> {
    fn scatter(
        &self,
        _rec: &HitRecord,
        _ray: &Ray,
        _dist: &Uniform<f64>,
        _rng: &mut ThreadRng,
    ) -> Option<(Ray, Colour)> {
        None
    }

    fn emitted(
        &self,
        u: f64,
        v: f64,
        p: Vec3,
        _dist: &Uniform<f64>,
        _rng: &mut ThreadRng,
    ) -> Colour {
        self.emit.value(u, v, p)
    }
}
