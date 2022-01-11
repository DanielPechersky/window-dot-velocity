use bevy::{app::Events, prelude::*};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

fn box_collider((hx, hy): (Real, Real)) -> ColliderShape {
    ColliderShape::compound(
        [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]]
            .map(|v| {
                let v: nalgebra::Unit<Vector<_>> = nalgebra::Unit::new_unchecked(v.into());
                (
                    Isometry::new(-v.component_mul(&Vector::from([hx, hy])), 0.0),
                    ColliderShape::halfspace(v),
                )
            })
            .into(),
    )
}

const WINDOW_COLLIDER: u32 = 1;

fn random_velocity_component() -> Real {
    (rand::random::<Real>() - 0.5) * 1.0
}

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Window;

fn setup(
    mut commands: Commands,
    windows: Res<Windows>,
    bevy_windows: Res<bevy::winit::WinitWindows>,
    rapier_config: Res<RapierConfiguration>,
) {
    let window = windows
        .get_primary()
        .and_then(|w| bevy_windows.get_window(w.id()))
        .unwrap();
    let monitor = window.current_monitor().unwrap();

    let radius = 0.03;

    for _ in 0..3 {
        use rand::seq::SliceRandom;
        const COLOURS: &[Color] = &[
            Color::RED,
            Color::ORANGE,
            Color::PINK,
            Color::BLUE,
            Color::GOLD,
        ];

        commands
            .spawn()
            .insert_bundle(GeometryBuilder::build_as(
                &shapes::Circle {
                    radius: radius * rapier_config.scale,
                    ..Default::default()
                },
                DrawMode::Fill(FillMode::color(
                    *COLOURS
                        .choose(&mut rand::thread_rng())
                        .expect("COLOURS is not empty"),
                )),
                Transform::default(),
            ))
            .insert_bundle(RigidBodyBundle::default())
            .insert_bundle(ColliderBundle {
                shape: ColliderShape::ball(radius).into(),
                material: ColliderMaterial::new(0.1, 1.0).into(),
                ..Default::default()
            })
            .insert(RigidBodyPositionSync::default())
            .insert(Ball);
    }

    // window
    commands
        .spawn()
        .insert_bundle(RigidBodyBundle {
            body_type: RigidBodyType::KinematicPositionBased.into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: box_collider({
                let size = window
                    .inner_size()
                    .to_logical::<Real>(window.scale_factor());
                (
                    size.width / 2. / rapier_config.scale,
                    size.height / 2. / rapier_config.scale,
                )
            })
            .into(),
            material: ColliderMaterial::new(0.8, 0.3).into(),
            flags: ColliderFlags {
                collision_groups: InteractionGroups::new(WINDOW_COLLIDER, WINDOW_COLLIDER),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        })
        .insert(RigidBodyPositionSync::default())
        .insert(Window);

    // // monitor bounds
    // commands.spawn().insert_bundle(ColliderBundle {
    //     shape: box_collider((
    //         monitor.size().width as Real / 2.0,
    //         monitor.size().height as Real / 2.0,
    //     ))
    //     .into(),
    //     ..Default::default()
    // });

    // commands
    //     .spawn()
    //     .insert_bundle(RigidBodyBundle {
    //         body_type: RigidBodyType::Static.into(),
    //         ..Default::default()
    //     })
    //     .insert_bundle(ColliderBundle {
    //         shape: ColliderShape::cuboid(100.0, 30.0).into(),
    //         position: [0.0, -100.0].into(),
    //         ..Default::default()
    //     })
    //     .insert(ColliderDebugRender::default())
    //     .insert(ColliderPositionSync::default());

    commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d());
}

fn window_center(window: &winit::window::Window) -> Point<Real> {
    let top_left = window
        .inner_position()
        .unwrap()
        .to_logical::<Real>(window.scale_factor());
    let size = window
        .inner_size()
        .to_logical::<Real>(window.scale_factor());

    // bevy's coordinate system has origin on the bottom left
    let monitor = window.current_monitor().unwrap();
    let monitor_height: Real = monitor.size().to_logical(monitor.scale_factor()).height;
    let bottom_left = [top_left.x, monitor_height - top_left.y - size.height];

    Point::<_>::from(bottom_left) + Vector::<_>::from([size.width, size.height]) / 2.
}

fn adjust_window_position(
    windows: Res<Windows>,
    mut window_physics: Query<&mut RigidBodyPositionComponent, With<Window>>,
    mut camera: Query<&mut Transform, With<Camera>>,
    bevy_windows: Res<bevy::winit::WinitWindows>,
    rapier_config: Res<RapierConfiguration>,
) {
    let mut window_physics = window_physics.single_mut();
    let window = windows
        .get_primary()
        .and_then(|w| bevy_windows.get_window(w.id()))
        .unwrap();
    let mut camera = camera.single_mut();
    let [x, y]: [Real; 2] = window_center(window).into();
    window_physics.next_position = Isometry::new(
        [x / rapier_config.scale, y / rapier_config.scale].into(),
        0.0,
    );
    camera.translation = [x, y, camera.translation.z].into();
}

fn resize_update(
    resized_events: Res<Events<bevy::window::WindowResized>>,
    mut window_physics: Query<&mut ColliderShapeComponent, With<Window>>,
    rapier_config: Res<RapierConfiguration>,
) {
    let mut window_physics = window_physics.single_mut();
    for event in resized_events.get_reader().iter(&resized_events) {
        *window_physics = box_collider((
            event.width / 2. / rapier_config.scale,
            event.height / 2. / rapier_config.scale,
        ))
        .into();
    }
}

pub fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "window.velocity".to_string(),
            width: 600.,
            height: 400.,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::GRAY))
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(RapierConfiguration {
            scale: 1500.,
            // gravity: Vector::zeros(),
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_system(adjust_window_position)
        .add_system(resize_update)
        .run();
}
