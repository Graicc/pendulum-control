use bevy::{input::mouse::MouseMotion, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use std::f32::consts::PI;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.9, 0.3, 0.6)))
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_startup_system(add_pendulum)
        .add_system(ui_example)
        // .add_system(control_pendulum_keyboard)
        .add_system(control_pendulum_mouse)
        .add_system(move_pendulum)
        .add_system(draw_pendulum)
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
}

const DT: f32 = 0.05;
const G: f32 = 9.8;

fn add_pendulum(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        Pendulum {
            a: PI,
            da: 0.01,
            length: 10.0,
            control_power: 5.0,
            ..default()
        },
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(10.).into()).into(),
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
        let clamped_control = pendulum.control.clamp(-1.0, 1.0);

        pendulum.da += (acceleration(&pendulum, G) + clamped_control * pendulum.control_power) * DT;
        pendulum.a += pendulum.da * DT;

        pendulum.control = 0.0;
    }
}

fn draw_pendulum(mut query: Query<(&mut Transform, &Pendulum)>) {
    for (mut transform, pendulum) in query.iter_mut() {
        let (x, y) = pendulum.to_rectangular();
        transform.translation = Vec3::new(x * 10.0, y * 10.0, 0.0);
    }
}

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
        pendulum.control = input;
    }
}

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
        pendulum.control = input;
    }
}

fn ui_example(mut egui_context: ResMut<EguiContext>, mut query: Query<&mut Pendulum>) {
    egui::Window::new("Pendulum settings").show(egui_context.ctx_mut(), |ui| {
        for mut pendulum in query.iter_mut() {
            ui.label("Pendulum");
            ui.add(egui::Slider::new(&mut pendulum.length, 0.0..=20.0).text("length"));
            ui.add(
                egui::Slider::new(&mut pendulum.control_power, 0.0..=20.0).text("control_power"),
            );

            if ui.button("Reset").clicked() {
                pendulum.a = PI;
                pendulum.da = 0.05;
            }

            // ui.label(format!("{pendulum:?}"));
        }
    });
}
