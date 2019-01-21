//  This file is part of Sulis, a turn based RPG written in Rust.
//  Copyright 2018 Jared Stephen
//
//  Sulis is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  Sulis is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License
//  along with Sulis.  If not, see <http://www.gnu.org/licenses/>

use rlua::{self, Context, UserData, UserDataMethods};

use sulis_core::resource::ResourceSet;
use sulis_core::util::ExtInt;

use crate::{GameState};
use crate::animation::{Anim, self};
use crate::animation::particle_generator::{Dist, Param, DistParam, DistParam2D, GeneratorModel};
use crate::script::{CallbackData, Result};

/// A flexible animation type, which can be used to create particle effects, simple
/// frame based animations, or anything in between.
/// Typically created by `ScriptEntity:create_particle_generator`
///
/// # `activate()`
/// Activates and applies this animation.
///
/// # `param(value: Float, dt: Float (Optional), d2t: Float (Optional),
/// d3t: Float (Optional))`
/// Creates a param which can then be passed to one of the various configuration
/// methods accepting a param in this script generator.  An initial `value` must
/// be specified - all other values are optional and default to zero.  `dt` is
/// the speed coefficient, `d2t` is the acceleration coefficient, and `d3t` is
/// the jerk coefficient.
///
/// # `dist_param(value: Dist, dt: Dist (Optional), d2t: Dist (Optional),
/// d3t: Dist (Optional))`
/// Creates a `dist_param`, which is a `param` where each component is a
/// distribution that is randomly selected from.  See `param`.
///
/// # `zero_dist() -> Dist`
/// Creates a new Dist with a fixed value of zero.
///
/// # `fixed_dist(value: Float) -> Dist`
/// Creates a dist which always returns the specified fixed `value`
///
/// # `uniform_dist(min: Float, max: Float) -> Dist`
/// Creates a dist which randomly generates a value between `min` and `max` in a uniform manner.
///
/// # `angular_dist(min_angle: Float, max_angle: Float, min_magnitude: Float,
/// max_magnitude: Float)`
/// Creates a dist which randomly generates a direction and magnitude for a vector.  As this is a
/// two component dist, may only be used when configuring the particle position distribution.
///
/// # `set_blocking(blocking: Bool)`
/// Sets whether this animation is `blocking`.  By default, animations with infinite time (or those
/// attached to an effect) are not blocking, but effects that have a fixed duration are. This
/// method allows you to change this as needed.
///
/// # `set_draw_below_entities()`
/// Sets this animation to perform drawing below the entity layer in the main view
///
/// # `set_draw_above_entities()`
/// Sets this animation to perform drawing above the entity layer in the main view.
///
/// # `set_initial_gen(value: Float)`
/// Sets the number of particles that should immediately be generated by this animation,
/// on its first frame.  This "jump-starts" the animation.
///
/// # `set_gen_rate(value: Param)`
/// Sets the number of particles generated each second.
///
/// # `set_moves_with_parent()`
/// By default, animations stay in the position where they were created, subjected to their
/// position.  With this set, the animation will follow along with the parent's position,
/// with the animation's position on top of this.
///
/// # `set_position(x: Param, y: Param)`
/// Sets the `x` and `y` coordinates of this animation's overall position.  Each time a particle
/// is generated, the particle's position is added to the generator position.
///
/// # `set_rotation(angle: Param)`
/// Sets an `angle` rotation (in radians) for all particles in this animation.  The rotation
/// is currently being done in software for convenience, so this is not suitable for
/// animations with many particles.
///
/// # `set_color(r: Param, g: Param, b: Param, a: Param (Optional))`
/// Sets the color which all particles in this animation are drawn using.  The `a` alpha
/// component is optional and defaults to a fixed value of 1.0.  Each component Param
/// should yield values between 0.0 and 1.0
///
/// # `set_alpha(a: Param)`
/// Sets the alpha color component for all particles in this animation.  The value should
/// be between 0.0 and 1.0
///
/// # `set_completion_callback(callback: CallbackData)`
/// Sets the specified `callback` to be called when this animation completes.
///
/// # `add_callback(callback: CallbackData, time: Float)`
/// Sets the specified `callback` to be called after the specified `time` has elapsed,
/// in seconds.
///
/// # `set_particle_position_dist(x: DistParam, y: DistParam (Optional))`
/// Sets the position distribution of particles generated by this animation.  If both
/// `x` and `y` are passed, then seperate `DistParam`s are used for each component.
/// If only `x` is passed, then the same component is used for both.  In the case of
/// `angular_dist` and potentially others in the future, this gives 2D control over
/// particle position.
///
/// # `set_particle_duration_dist(duration: Dist)`
/// Sets the length of time that particles exist for after being created, in seconds
///
/// # `set_particle_size_dist(width: Dist, height: Dist)`
/// Sets the size (where 1.0 equals 1 tile) of particles created by this animation.
///
/// #`set_particle_frame_time_offset_dist(value: Dist)`
/// Sets a frame offset time for each particle created by this animation.  This is only
/// useful for particles that are using a `TimerImage`.  When `value` is a random
/// distribution, all particles generated by this animation will cease to be synced,
/// and instead all start, loop, and/or stop at random times with respect to one another.

#[derive(Clone)]
pub struct ScriptParticleGenerator {
    parent: usize,
    image: String,
    completion_callback: Option<CallbackData>,
    callbacks: Vec<(f32, CallbackData)>,
    model: GeneratorModel,
}

impl ScriptParticleGenerator {
    pub fn new(parent: usize, image: String, duration_millis: ExtInt) -> ScriptParticleGenerator {
        let mgr = GameState::turn_manager();
        let owner = mgr.borrow().entity(parent);
        let x = owner.borrow().location.x as f32 + owner.borrow().size.width as f32 / 2.0;
        let y = owner.borrow().location.y as f32 + owner.borrow().size.height as f32 / 2.0;

        let model = GeneratorModel::new(duration_millis, x, y);

        ScriptParticleGenerator {
            parent,
            image,
            completion_callback: None,
            callbacks: Vec::new(),
            model,
        }
    }

    pub fn new_anim(parent: usize, image: String, duration_millis: ExtInt) -> ScriptParticleGenerator {
        let mut pgen = ScriptParticleGenerator::new(parent, image, duration_millis);
        pgen.model.initial_overflow = 1.0;
        pgen.model.gen_rate = Param::fixed(0.0);
        pgen
    }

    pub fn owned_model(&self) -> GeneratorModel {
        self.model.clone()
    }
}

impl UserData for ScriptParticleGenerator {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("activate", &activate);
        methods.add_method("param", &param);
        methods.add_method("dist_param", &dist_param);
        methods.add_method("zero_dist", |_, _, _: ()| Ok(Dist::create_fixed(0.0)));
        methods.add_method("fixed_dist", |_, _, value: f32| Ok(Dist::create_fixed(value)));
        methods.add_method("uniform_dist", |_, _, (min, max): (f32, f32)| Ok(Dist::create_uniform(min, max)));
        methods.add_method("angular_dist", |_, _, (min_a, max_a, min_s, max_s): (f32, f32, f32, f32)| {
            Ok(Dist::create_angular(min_a, max_a, min_s, max_s))
        });
        methods.add_method_mut("set_blocking", |_, gen, block: bool| {
            gen.model.is_blocking = block;
            Ok(())
        });
        methods.add_method_mut("set_draw_below_entities", |_, gen, _: ()| {
            gen.model.draw_above_entities = false;
            Ok(())
        });
        methods.add_method_mut("set_draw_above_entities", |_, gen, _: ()| {
            gen.model.draw_above_entities = true;
            Ok(())
        });
        methods.add_method_mut("set_initial_gen", |_, gen, value: f32| {
            gen.model.initial_overflow = value;
            Ok(())
        });
        methods.add_method_mut("set_moves_with_parent", |_, gen, _args: ()| {
            gen.model.moves_with_parent = true;
            Ok(())
        });
        methods.add_method_mut("set_gen_rate", |_, gen, rate: Param| {
            gen.model.gen_rate = rate;
            Ok(())
        });
        methods.add_method_mut("set_position", |_, gen, (x, y): (Param, Param)| {
            gen.model.position = (x, y);
            Ok(())
        });
        methods.add_method_mut("set_rotation", |_, gen, rotation: Param| {
            gen.model.rotation = Some(rotation);
            Ok(())
        });
        methods.add_method_mut("set_color", |_, gen, (r, g, b, a): (Param, Param, Param, Option<Param>)| {
            gen.model.red = r;
            gen.model.green = g;
            gen.model.blue = b;
            if let Some(a) = a {
                gen.model.alpha = a;
            }
            Ok(())
        });
        methods.add_method_mut("set_alpha", |_, gen, a: Param| {
            gen.model.alpha = a;
            Ok(())
        });
        methods.add_method_mut("set_completion_callback", |_, gen, cb: CallbackData| {
            gen.completion_callback = Some(cb);
            Ok(())
        });
        methods.add_method_mut("add_callback", |_, gen, (cb, time): (CallbackData, f32)| {
            gen.callbacks.push((time, cb));
            Ok(())
        });
        methods.add_method_mut("set_particle_position_dist", |_, gen, (x, y): (DistParam, Option<DistParam>)| {
            gen.model.particle_position_dist = Some(DistParam2D::new(x, y));
           Ok(())
        });
        methods.add_method_mut("set_particle_duration_dist", |_, gen, value: Dist| {
            gen.model.particle_duration_dist = Some(value);
            Ok(())
        });
        methods.add_method_mut("set_particle_size_dist", |_, gen, (width, height): (Dist, Dist)| {
            gen.model.particle_size_dist = Some((width, height));
            Ok(())
        });
        methods.add_method_mut("set_particle_frame_time_offset_dist", |_, gen, value: Dist| {
            gen.model.particle_frame_time_offset_dist = Some(value);
            Ok(())
        });
    }
}

fn dist_param(_lua: Context, _: &ScriptParticleGenerator,
              (value, dt, d2t, d3t) : (Dist, Option<Dist>, Option<Dist>, Option<Dist>)) -> Result<DistParam> {
    if dt.is_none() {
        Ok(DistParam::new(value, Dist::create_fixed(0.0), Dist::create_fixed(0.0), Dist::create_fixed(0.0)))
    } else if d2t.is_none() {
        Ok(DistParam::new(value, dt.unwrap(), Dist::create_fixed(0.0), Dist::create_fixed(0.0)))
    } else if d3t.is_none() {
        Ok(DistParam::new(value, dt.unwrap(), d2t.unwrap(), Dist::create_fixed(0.0)))
    } else {
        Ok(DistParam::new(value, dt.unwrap(), d2t.unwrap(), d3t.unwrap()))
    }
}

pub fn param<T>(_lua: Context, _: &T,
         (value, dt, d2t, d3t): (f32, Option<f32>, Option<f32>, Option<f32>)) -> Result<Param> {
    if dt.is_none() {
        Ok(Param::fixed(value))
    } else if d2t.is_none() {
        Ok(Param::with_speed(value, dt.unwrap()))
    } else if d3t.is_none() {
        Ok(Param::with_accel(value, dt.unwrap(), d2t.unwrap()))
    } else {
        Ok(Param::with_jerk(value, dt.unwrap(), d2t.unwrap(), d3t.unwrap()))
    }
}

fn activate(_lua: Context, gen: &ScriptParticleGenerator, _args: ()) -> Result<()> {
    let pgen = create_pgen(gen, gen.model.clone())?;

    GameState::add_animation(pgen);

    Ok(())
}

pub fn create_surface_pgen(gen: &ScriptParticleGenerator, x: i32, y: i32) -> Result<Anim> {
    let mut model = gen.model.clone();
    let x_param = model.position.0.offset(x as f32);
    let y_param = model.position.1.offset(y as f32);
    model.position = (x_param, y_param);

    create_pgen(gen, model)
}

pub fn create_pgen(gen: &ScriptParticleGenerator, model: GeneratorModel) -> Result<Anim> {
    let mgr = GameState::turn_manager();
    let parent = mgr.borrow().entity(gen.parent);

    let image = match ResourceSet::get_image(&gen.image) {
        Some(image) => image,
        None => {
            warn!("Unable to locate image '{}' for particle generator", gen.image);
            return Err(rlua::Error::FromLuaConversionError {
                from: "ScriptParticleGenerator",
                to: "ParticleGenerator",
                message: Some("Image not found".to_string()),
            });
        }
    };

    let mut pgen = animation::particle_generator::new(&parent, image, model);

    if let Some(ref cb) = gen.completion_callback {
        pgen.add_completion_callback(Box::new(cb.clone()));
    }

    for &(time, ref cb) in gen.callbacks.iter() {
        pgen.add_update_callback(Box::new(cb.clone()), (time * 1000.0) as u32);
    }

    Ok(pgen)
}
