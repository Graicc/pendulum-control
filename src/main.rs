use std::f32::consts::PI;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.9, 0.3, 0.6)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(add_pendulum)
        .add_system(move_pendulum)
        .run();
}

#[derive(Component)]
struct Pendulum {
    a: f32,
    da: f32,
    length: f32,
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
            length: 100.0,
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

fn move_pendulum(mut query: Query<(&mut Transform, &mut Pendulum)>) {
    for (mut transform, mut pendulum) in query.iter_mut() {
        pendulum.da += acceleration(&pendulum, G);
        pendulum.a += pendulum.da * DT;

        let (x, y) = pendulum.to_rectangular();
        transform.translation = Vec3::new(x, y, 0.0);
    }
}
