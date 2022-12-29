use bevy::{input::mouse::MouseMotion, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_egui::{
    egui::{
        self,
        plot::{Legend, Line, Plot, PlotPoints},
        Id,
    },
    EguiContext, EguiPlugin,
};
use bevy_prototype_debug_lines::*;
use lqr::LQRController;
use nalgebra::{ArrayStorage, Const, Matrix, Matrix1, Matrix1x2, Matrix2, Matrix2x1};
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
        .add_system(control_pendulum_lqr)
        .add_system(move_pendulum)
        .add_system(draw_pendulum)
        .add_system(debug_draw)
        .add_system(history)
        .run();
}

#[derive(Component, Debug)]
struct Pendulum {
    a: f32,
    da: f32,
    length: f32,
    friction: f32,
    control: f32,
    control_power: f32,
    control_history: Vec<f32>,
    offset: Vec3,
}

impl Default for Pendulum {
    fn default() -> Self {
        Self {
            a: PI + 0.5,
            da: 0.1,
            length: 10.0,
            friction: 0.0,
            control: Default::default(),
            control_power: 5.0,
            control_history: Default::default(),
            offset: Default::default(),
        }
    }
}

impl Pendulum {
    fn from_offset(x: f32, y: f32) -> Self {
        Pendulum {
            offset: Vec3::new(x, y, 0.0),
            ..default()
        }
    }

    fn to_rectangular(&self) -> (f32, f32) {
        to_rectangular(self.length, self.a)
    }

    fn set_control(&mut self, value: f32) {
        self.control = value.clamp(-1.0, 1.0);
        // self.control = value;
    }

    fn get_system(&self) -> (A, B) {
        let dt2 = DT.powf(2.0);

        let a = Matrix2::<f32>::new(
            1.0 + G / (2.0 * self.length) * dt2,
            DT - self.friction / 2.0 * dt2,
            G / self.length * DT,
            1.0 - self.friction * DT,
        );

        let b = Matrix2x1::new(self.control_power / 2.0 * dt2, self.control_power * DT);

        (a, b)
    }
}

fn to_rectangular(length: f32, angle: f32) -> (f32, f32) {
    let x = length * angle.sin();
    let y = -length * angle.cos();
    (x, y)
}

#[derive(Component, Default)]
struct PID {
    set_point: f32,
    proportional_gain: f32,
    integral_gain: f32,
    derivative_gain: f32,
    accumulator: f32,
    accumulator_enabled: bool,
    error_history: Vec<f32>,
    accumulator_history: Vec<f32>,
}

type A = Matrix<f32, Const<2>, Const<2>, ArrayStorage<f32, 2, 2>>;
type B = Matrix<f32, Const<2>, Const<1>, ArrayStorage<f32, 2, 1>>;
type Q = Matrix<f32, Const<2>, Const<2>, ArrayStorage<f32, 2, 2>>;
type R = Matrix<f32, Const<1>, Const<1>, ArrayStorage<f32, 1, 1>>;

#[derive(Component)]
struct LQR {
    set_point: f32,
    a: A,
    b: B,
    q: Q,
    r: R,
}

impl LQR {
    fn set_gains(&mut self, pos_cost: f32, vel_cost: f32, power_cost: f32) {
        self.q = Q::new(pos_cost, 0.0, 0.0, vel_cost);
        self.r = R::new(power_cost);
    }
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
        Pendulum::from_offset(-7.0, 0.0),
        PID {
            set_point: PI,
            // set_point: 4.3,
            proportional_gain: -8.0,
            integral_gain: -5.5,
            derivative_gain: -4.0,
            ..default()
        },
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(1.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::WHITE)),
            transform: Transform::default(),
            ..default()
        },
    ));

    let p = Pendulum::from_offset(7.0, 0.0);

    let (a, b) = p.get_system();
    let q = Matrix2::identity();
    let r = Matrix1::identity();

    commands.spawn((
        p,
        LQR {
            set_point: PI,
            a,
            b,
            q,
            r,
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
    -gravity * pendulum.a.sin() / pendulum.length - pendulum.friction * pendulum.da
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
        transform.translation = Vec3::new(x, y, 0.0) + pendulum.offset;
    }
}

fn history(mut query: Query<(&mut Pendulum, Option<&mut PID>, Option<&mut LQR>)>) {
    for (mut pendulum, pid, lqr) in query.iter_mut() {
        let control = pendulum.control;
        pendulum.control_history.push(control);

        if let Some(mut pid) = pid {
            let error = pid.set_point - pendulum.a;
            pid.error_history.push(error);
            let acc = pid.accumulator;
            pid.accumulator_history.push(acc);
        }
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
        let error = pendulum.a - pid.set_point;
        let prop = error * pid.proportional_gain;

        // derivative
        let der = pendulum.da * pid.derivative_gain;

        let control = prop + der;

        // integral
        if error.abs() < 0.05 {
            pid.accumulator_enabled = true;
        }

        if pid.accumulator_enabled {
            pid.accumulator += error * pid.integral_gain * DT;
            // pid.accumulator = pid.accumulator.clamp(-1.0, 1.0);
            pid.accumulator = pid
                .accumulator
                .clamp((-1.0 - control).min(0.0), (1.0 - control).max(0.0));
        }

        let control = prop + pid.accumulator + der;

        pendulum.set_control(control);
    }
}

fn control_pendulum_lqr(mut query: Query<(&mut Pendulum, &mut LQR)>) {
    for (mut pendulum, lqr) in query.iter_mut() {
        let mut controller = LQRController::new().unwrap();

        let k: Matrix1x2<_> = controller
            .compute_gain(&lqr.a, &lqr.b, &lqr.q, &lqr.r, 1e-7)
            .unwrap();

        let x = Matrix2x1::new(pendulum.a - lqr.set_point, pendulum.da - 0.0);

        let u = -k * x;

        pendulum.set_control(*u.index(0));
    }
}

fn debug_draw(
    mut lines: ResMut<DebugLines>,
    query: Query<(&Pendulum, Option<&PID>, Option<&LQR>)>,
) {
    for (pendulum, pid, lqr) in query.iter() {
        let (x, y) = pendulum.to_rectangular();
        lines.line(pendulum.offset, Vec3::new(x, y, 0.0) + pendulum.offset, 0.0);

        if let Some(pid) = pid {
            let (x, y) = to_rectangular(pendulum.length, pid.set_point);
            lines.line(pendulum.offset, Vec3::new(x, y, 0.0) + pendulum.offset, 0.0);
        }

        if let Some(lqr) = lqr {
            let (x, y) = to_rectangular(pendulum.length, lqr.set_point);
            lines.line(pendulum.offset, Vec3::new(x, y, 0.0) + pendulum.offset, 0.0);
        }
    }
}

fn to_points(v: &Vec<f32>) -> PlotPoints {
    v.iter()
        .enumerate()
        .map(|(i, v)| [(i as f64) * (DT as f64), *v as f64])
        .collect()
}

fn ui_example(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<(&mut Pendulum, Option<&mut PID>, Option<&mut LQR>)>,
) {
    for (i, (mut pendulum, mut pid, mut lqr)) in query.iter_mut().enumerate() {
        egui::Window::new("Pendulum settings")
            .id(Id::new(i))
            .resizable(true)
            .default_pos((
                if i % 2 == 0 { 20.0 } else { 1100.0 },
                20.0 + 100.0 * (i / 2) as f32,
            ))
            .show(egui_context.ctx_mut(), |ui| {
                ui.label("Pendulum");
                ui.add(egui::Slider::new(&mut pendulum.length, 0.0..=20.0).text("length"));
                ui.add(
                    egui::Slider::new(&mut pendulum.control_power, 0.0..=20.0)
                        .text("Control power"),
                );

                ui.add(egui::Slider::new(&mut pendulum.a, 0.0..=2.0 * PI).text("Angle"));
                ui.add(egui::Slider::new(&mut pendulum.da, -10.0..=10.0).text("Speed"));

                ui.label(format!("{}", pendulum.control));

                if ui.button("Reset").clicked() {
                    let template = Pendulum::default();
                    pendulum.a = template.a;
                    pendulum.da = template.da;
                    pendulum.control_history = Vec::new();
                    if let Some(pid) = &mut pid {
                        pid.accumulator = 0.0;
                        pid.accumulator_enabled = false;
                        pid.error_history = Vec::new();
                        pid.accumulator_history = Vec::new();
                    }
                }

                let slider_range = 10.0;

                let mut lines = Vec::new();
                let control_points: PlotPoints = to_points(&pendulum.control_history);
                lines.push(Line::new(control_points).name("Control"));

                if let Some(mut pid) = pid {
                    ui.separator();
                    ui.label("PID");
                    ui.label(format!("Error: {}", pendulum.a - pid.set_point));
                    // TODO: There's probably a better way to do this
                    let old_set_point = pid.set_point;
                    ui.add(egui::Slider::new(&mut pid.set_point, 0.0..=2.0 * PI).text("Set point"));
                    if pid.set_point != old_set_point {
                        pid.accumulator = 0.0;
                        pid.accumulator_enabled = false;
                    }
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

                    let error_points: PlotPoints = to_points(&pid.error_history);
                    let accumulator_points: PlotPoints = to_points(&pid.accumulator_history);

                    lines.push(Line::new(error_points).name("Error"));
                    lines.push(Line::new(accumulator_points).name("Accumulator"));
                }

                if let Some(mut lqr) = lqr {
                    ui.separator();
                    ui.label("LQR");

                    ui.label(format!("Error: {}", pendulum.a - PI));
                }

                Plot::new("My Plot")
                    .legend(Legend::default())
                    .view_aspect(2.0)
                    .show(ui, |plot_ui| {
                        for line in lines {
                            plot_ui.line(line);
                        }
                    });
            });
    }
}
