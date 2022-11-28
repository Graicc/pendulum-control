use bevy::{input::mouse::MouseMotion, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_prototype_debug_lines::*;
use std::f32::consts::PI;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.9, 0.3, 0.6)))
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(DebugLinesPlugin::default())
        .add_startup_system(add_pendulum)
        .add_system(ui_example)
        // .add_system(control_pendulum_keyboard)
        // .add_system(control_pendulum_mouse)
        .add_system(control_pendulum_pid)
        .add_system(move_pendulum)
        .add_system(draw_pendulum)
        .add_system(debug_draw)
        .run();
}

#[derive(Component, Default, Debug)]
struct Pendulum {
    a: f32,
    da: f32,
    length: f32,
    control: f32,
    control_power: f32,
}

impl Pendulum {
    fn to_rectangular(&self) -> (f32, f32) {
        let x = self.length * self.a.sin();
        let y = self.length * -self.a.cos();
        (x, y)
    }

    fn set_control(&mut self, value: f32) {
        self.control = value.clamp(-1.0, 1.0);
    }
}

#[derive(Component)]
struct PID {
    set_point: f32,
    proportional_gain: f32,
    integral_gain: f32,
    derivative_gain: f32,
    accumulator: f32,
}

const DT: f32 = 0.05;
const G: f32 = 9.8;

fn add_pendulum(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scale *= 0.1;
    commands.spawn(camera_bundle);

    commands.spawn((
        Pendulum {
            a: PI,
            da: 0.01,
            length: 10.0,
            control_power: 5.0,
            ..default()
        },
        PID {
            accumulator: 0.0,
            set_point: PI,
            proportional_gain: -8.0,
            integral_gain: 0.0,
            derivative_gain: -4.0,
        },
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(1.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::WHITE)),
            transform: Transform::default(),
            ..default()
        },
    ));
}

fn acceleration(pendulum: &Pendulum, gravity: f32) -> f32 {
    -gravity * pendulum.a.sin() / pendulum.length - 0.01 * pendulum.da
}

fn move_pendulum(mut query: Query<&mut Pendulum>) {
    for mut pendulum in query.iter_mut() {
        pendulum.da +=
            (acceleration(&pendulum, G) + pendulum.control * pendulum.control_power) * DT;
        pendulum.a += pendulum.da * DT;
    }
}

fn draw_pendulum(mut query: Query<(&mut Transform, &Pendulum)>) {
    for (mut transform, pendulum) in query.iter_mut() {
        let (x, y) = pendulum.to_rectangular();
        transform.translation = Vec3::new(x, y, 0.0);
    }
}

#[allow(dead_code)]
fn control_pendulum_keyboard(keys: Res<Input<KeyCode>>, mut query: Query<&mut Pendulum>) {
    let mut input = 0.0;
    input += if keys.pressed(KeyCode::Left) {
        -1.0
    } else {
        0.0
    };
    input += if keys.pressed(KeyCode::Right) {
        1.0
    } else {
        0.0
    };

    for mut pendulum in query.iter_mut() {
        pendulum.set_control(input)
    }
}

#[allow(dead_code)]
fn control_pendulum_mouse(
    buttons: Res<Input<MouseButton>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut query: Query<&mut Pendulum>,
) {
    let mut acc = 0.0;
    for ev in motion_evr.iter() {
        acc += ev.delta.x;
    }

    let input = acc / 5.0;

    if !buttons.pressed(MouseButton::Left) {
        return;
    };

    for mut pendulum in query.iter_mut() {
        pendulum.set_control(input)
    }
}

fn control_pendulum_pid(mut query: Query<(&mut Pendulum, &mut PID)>) {
    for (mut pendulum, mut pid) in query.iter_mut() {
        // proportional
        let prop = (pendulum.a - pid.set_point) * pid.proportional_gain;

        // integral
        pid.accumulator += (pendulum.a - pid.set_point) * pid.integral_gain * DT;

        // derivative
        let der = pendulum.da * pid.derivative_gain;

        let control = prop + pid.accumulator + der;

        pendulum.set_control(control);
        debug!(pendulum.control);
    }
}

fn debug_draw(mut lines: ResMut<DebugLines>, query: Query<(&Pendulum, &PID)>) {
    for (pendulum, pid) in query.iter() {
        let (x, y) = pendulum.to_rectangular();
        let zero = Vec3::new(0.0, 0.0, 0.0);
        lines.line(zero, Vec3::new(x, y, 0.0), 0.0);
        let x = pendulum.length * pid.set_point.sin();
        let y = pendulum.length * -pid.set_point.cos();
        lines.line(zero, Vec3::new(x, y, 0.0), 0.0);
    }
}

fn ui_example(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<(&mut Pendulum, Option<&mut PID>)>,
) {
    egui::Window::new("Pendulum settings").show(egui_context.ctx_mut(), |ui| {
        for (mut pendulum, mut pid) in query.iter_mut() {
            ui.label("Pendulum");
            ui.add(egui::Slider::new(&mut pendulum.length, 0.0..=20.0).text("length"));
            ui.add(
                egui::Slider::new(&mut pendulum.control_power, 0.0..=20.0).text("control_power"),
            );

            ui.label(format!("{}", pendulum.control));

            if ui.button("Reset").clicked() {
                pendulum.a = PI;
                pendulum.da = 0.05;
                if let Some(pid) = &mut pid {
                    pid.accumulator = 0.0;
                }
            }

            let slider_range = 10.0;

            if let Some(mut pid) = pid {
                ui.add_space(10.0);
                ui.label("PID");
                ui.label(format!("Error: {}", pendulum.a - pid.set_point));
                ui.add(egui::Slider::new(&mut pid.set_point, 0.0..=2.0 * PI).text("Set point"));
                ui.add(
                    egui::Slider::new(&mut pid.proportional_gain, -slider_range..=slider_range)
                        .text("Proportional gain"),
                );
                ui.add(
                    egui::Slider::new(&mut pid.integral_gain, -slider_range..=slider_range)
                        .text("Integral gain"),
                );
                ui.add(
                    egui::Slider::new(&mut pid.derivative_gain, -slider_range..=slider_range)
                        .text("Derivative gain"),
                );
            }
        }
    });
}
