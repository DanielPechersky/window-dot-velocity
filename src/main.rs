#![windows_subsystem = "windows"]

use bevy::{prelude::*, window::WindowResized, winit::WinitWindows};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use winit::dpi::{LogicalPosition, LogicalSize};

fn box_collider([hx, hy]: [Real; 2]) -> ColliderShape {
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

const WINDOW_INNER: u32 = 1;

#[derive(Component, Clone, Copy)]
enum Window {
    Bouncing,
    Dragging(LogicalPosition<Real>),
    Static,
}

impl Default for Window {
    fn default() -> Self {
        Self::Static
    }
}

#[derive(Component)]
struct WindowWalls;

#[derive(Clone, Copy)]
struct CoordConverter {
    monitor_height: Real, // in logical units
    physics_scale: Real,
}

impl CoordConverter {
    fn flip(&self, mut p: LogicalPosition<Real>) -> LogicalPosition<Real> {
        p.y = self.monitor_height - p.y;
        p
    }

    fn to_physics_point(&self, p: LogicalPosition<Real>) -> Point<Real> {
        self.to_physics_vec(<[Real; 2]>::from(self.flip(p)).into())
            .into()
    }

    fn to_physics_vec(&self, p: LogicalSize<Real>) -> Vector<Real> {
        Vector::from(<[_; 2]>::from(p)) / self.physics_scale
    }

    fn to_logical_winit_position(&self, v: Point<Real>) -> LogicalPosition<Real> {
        self.flip(<[Real; 2]>::from(self.to_logical_size(v.coords)).into())
    }

    fn to_logical_size(&self, v: Vector<Real>) -> LogicalSize<Real> {
        <[_; 2]>::from(v * self.physics_scale).into()
    }

    fn from_bevy_winit(&self, v: Vec2) -> LogicalPosition<Real> {
        self.flip(<[Real; 2]>::from(v).into())
    }
}

fn setup(
    mut commands: Commands,
    windows: Res<Windows>,
    bevy_windows: Res<WinitWindows>,
    rapier_config: Res<RapierConfiguration>,
) {
    let window = windows
        .get_primary()
        .and_then(|w| bevy_windows.get_window(w.id()))
        .unwrap();
    let monitor = window.current_monitor().unwrap();
    let monitor_height: Real = monitor.size().to_logical(monitor.scale_factor()).height;

    let converter = CoordConverter {
        monitor_height: monitor_height,
        physics_scale: rapier_config.scale,
    };
    commands.insert_resource(converter);

    let camera = commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d())
        .id();

    // window
    let walls = commands
        .spawn_bundle(ColliderBundle {
            shape: box_collider({
                let size = window
                    .inner_size()
                    .to_logical::<Real>(window.scale_factor());
                let size = converter.to_physics_vec(size);
                (size / 2.).into()
            })
            .into(),
            material: ColliderMaterial::new(0.8, 0.3).into(),
            flags: ColliderFlags {
                collision_groups: InteractionGroups::new(u32::MAX ^ WINDOW_INNER, u32::MAX),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        })
        .insert(WindowWalls)
        .id();

    commands
        .spawn()
        .insert_bundle(RigidBodyBundle {
            body_type: RigidBodyType::KinematicPositionBased.into(),
            mass_properties: RigidBodyMassPropsFlags::ROTATION_LOCKED.into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: {
                let size = window
                    .outer_size()
                    .to_logical::<Real>(window.scale_factor());
                let halfbounds = converter.to_physics_vec(size) / 2.;
                ColliderShape::cuboid(halfbounds[0], halfbounds[1]).into()
            },
            material: ColliderMaterial::new(0.8, 0.3).into(),
            flags: ColliderFlags {
                collision_groups: InteractionGroups::new(u32::MAX, WINDOW_INNER),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        })
        .insert(RigidBodyPositionSync::default())
        .insert(Window::default())
        .add_child(walls)
        .add_child(camera);

    // monitor
    let monitor_size = monitor.size().to_logical::<Real>(monitor.scale_factor());
    let monitor_size = converter.to_physics_vec(monitor_size);

    commands.spawn().insert_bundle(ColliderBundle {
        shape: box_collider((monitor_size / 2.).into()).into(),
        position: Isometry::new(monitor_size / 2., 0.).into(),
        material: ColliderMaterial::new(0.8, 0.3).into(),
        flags: ColliderFlags {
            collision_groups: InteractionGroups::new(u32::MAX, WINDOW_INNER),
            ..Default::default()
        }
        .into(),
        ..Default::default()
    });

    for _ in 0..5 {
        use rand::seq::SliceRandom;
        const COLOURS: &[Color] = &[
            Color::RED,
            Color::ORANGE,
            Color::PINK,
            Color::BLUE,
            Color::GOLD,
        ];

        let size = rand::random::<Real>() * 0.06 + 0.01;

        enum Choice {
            Circle,
            Square,
        }

        let mode = DrawMode::Fill(FillMode::color(
            *COLOURS
                .choose(&mut rand::thread_rng())
                .expect("COLOURS is not empty"),
        ));

        let (gbundle, cshape) = {
            match [Choice::Circle, Choice::Square]
                .choose(&mut rand::thread_rng())
                .unwrap()
            {
                Choice::Circle => (
                    GeometryBuilder::build_as(
                        &shapes::Circle {
                            radius: size * converter.physics_scale,
                            ..Default::default()
                        },
                        mode,
                        Transform::default(),
                    ),
                    ColliderShape::ball(size).into(),
                ),
                Choice::Square => (
                    GeometryBuilder::build_as(
                        &shapes::Rectangle {
                            extents: Vec2::from([size, size]) * converter.physics_scale,
                            origin: RectangleOrigin::Center,
                        },
                        mode,
                        Transform::default(),
                    ),
                    ColliderShape::cuboid(size / 2.0, size / 2.0),
                ),
            }
        };

        commands
            .spawn()
            .insert_bundle(gbundle)
            .insert_bundle(RigidBodyBundle {
                // ccd: RigidBodyCcd {
                //     ccd_thickness: size,
                //     ccd_max_dist: size * 2.,
                //     ccd_enabled: true,
                //     ..Default::default()
                // }
                // .into(),
                ..Default::default()
            })
            .insert_bundle(ColliderBundle {
                shape: cshape.into(),
                material: ColliderMaterial::new(0.3, 0.5).into(),
                flags: ColliderFlags {
                    collision_groups: InteractionGroups::new(u32::MAX ^ WINDOW_INNER, u32::MAX),
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            })
            .insert(RigidBodyPositionSync::default());
    }
}

fn window_background_indicates_state(mut background: ResMut<ClearColor>, window: Query<&Window>) {
    *background = match window.get_single().unwrap() {
        Window::Bouncing => ClearColor(Color::NAVY),
        Window::Dragging(_) => ClearColor(Color::DARK_GRAY),
        Window::Static => ClearColor(Color::GRAY),
    }
}

fn update_physics_or_application_window(
    windows: Res<Windows>,
    mut window_query: Query<(&Window, &mut RigidBodyPositionComponent), With<Window>>,
    bevy_windows: Res<WinitWindows>,
    converter: Res<CoordConverter>,
) {
    let (window_state, mut window_physics) = window_query.single_mut();
    let window = windows
        .get_primary()
        .and_then(|w| bevy_windows.get_window(w.id()))
        .unwrap();

    let size = window
        .outer_size()
        .to_logical::<Real>(window.scale_factor());
    let size = converter.to_physics_vec(size);
    let offset = Vector::from([size[0], -size[1]]) / 2.;

    match window_state {
        Window::Bouncing => {
            let center: Point<_> = window_physics.position.translation.vector.into();

            let top_left = center - offset;

            window.set_outer_position(converter.to_logical_winit_position(top_left));
        }
        Window::Static => {
            let top_left = window
                .inner_position()
                .unwrap()
                .to_logical::<Real>(window.scale_factor());
            let top_left = converter.to_physics_point(top_left);

            let center = top_left + offset;

            window_physics.next_position = Isometry::new(center.coords, 0.0);
        }
        Window::Dragging(_) => {}
    }
}

fn window_physics_type_update(
    mut window_query: Query<(&Window, &mut RigidBodyTypeComponent), Changed<Window>>,
) {
    if let Ok((window, mut rbtype)) = window_query.get_single_mut() {
        *rbtype = match window {
            Window::Bouncing => RigidBodyType::Dynamic,
            Window::Static | Window::Dragging(_) => RigidBodyType::KinematicPositionBased,
        }
        .into()
    }
}

// this doesn't update Window, also uses internal instead of external coordinates
fn resize_update(
    mut resized_events: EventReader<WindowResized>,
    mut window_query: Query<&mut ColliderShapeComponent, With<WindowWalls>>,
    converter: Res<CoordConverter>,
) {
    let mut window_physics = window_query.single_mut();
    for event in resized_events.iter() {
        let new_dims = converter.to_physics_vec([event.width, event.height].into());
        let new_dims = new_dims / 2.;
        *window_physics = box_collider(new_dims.into()).into();
    }
}

fn toggle_physics_on_spacebar(keys: Res<Input<KeyCode>>, mut window: Query<&mut Window>) {
    if keys.just_pressed(KeyCode::Space) {
        let mut window = window.single_mut();
        *window = match *window {
            Window::Static | Window::Dragging(_) => Window::Bouncing,
            Window::Bouncing => Window::Static,
        }
    }
}

fn clicking_freezes_window(
    mouse_button: Res<Input<MouseButton>>,
    mut window: Query<&mut Window>,
    windows: Res<Windows>,
    converter: Res<CoordConverter>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let mut window_state = window.single_mut();
        let window = windows.get_primary().unwrap();
        if let Some(p) = window.cursor_position() {
            *window_state = Window::Dragging(converter.from_bevy_winit(p));
        } else {
            debug!("Failed to get cursor for drag start")
        }
    }
}

fn dragging_flings_window(
    mouse_button: Res<Input<MouseButton>>,
    mut window: Query<(
        &mut Window,
        &mut RigidBodyVelocityComponent,
        &RigidBodyMassPropsComponent,
    )>,
    windows: Res<Windows>,
    converter: Res<CoordConverter>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        let (mut window_state, mut window_velocity, rbmp) = window.single_mut();
        let window = windows.get_primary().unwrap();
        if let Window::Dragging(prev) = *window_state {
            *window_state = Window::Bouncing;
            if let Some(curr) = window.cursor_position() {
                let prev = converter.to_physics_point(prev);
                let curr = converter.to_physics_point(converter.from_bevy_winit(curr));
                window_velocity.apply_impulse_at_point(&rbmp, (curr - prev) * 2.0, prev);
            } else {
                debug!("Failed to get cursor for drag end")
            }
        }
    }
}

struct WindowPhysicsPlugin;

impl Plugin for WindowPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RapierConfiguration {
            scale: 1500.,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_system(update_physics_or_application_window)
        .add_system(resize_update)
        .add_system(window_physics_type_update)
        .add_system(toggle_physics_on_spacebar)
        .add_system(clicking_freezes_window)
        .add_system(dragging_flings_window)
        .add_system(window_background_indicates_state);
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
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(WindowPhysicsPlugin)
        .run();
}
