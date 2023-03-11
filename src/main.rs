#![windows_subsystem = "windows"]

use bevy::{prelude::*, window::WindowResized, winit::WinitWindows};
use bevy_prototype_lyon as blyon;
use bevy_rapier2d::prelude::*;
use winit::dpi::{LogicalPosition, LogicalSize};

const PIXELS_PER_METER: f32 = 1500.0 / 2.;

fn box_collider([hx, hy]: [Real; 2]) -> Collider {
    Collider::compound(
        [Vect::X, -Vect::X, Vect::Y, -Vect::Y]
            .map(|v| {
                (
                    -v * Vect::new(hx, hy),
                    Rot::default(),
                    Collider::halfspace(v).unwrap(),
                )
            })
            .into(),
    )
}

#[derive(Component, Clone, Copy)]
enum WindowState {
    Bouncing,
    Dragging(LogicalPosition<Real>),
    Static,
}

impl Default for WindowState {
    fn default() -> Self {
        Self::Static
    }
}

#[derive(Component)]
struct WindowWalls;

#[derive(Resource, Clone, Copy)]
struct CoordConverter {
    monitor_height: Real, // in logical units
                          // physics_scale: Real,
}

impl CoordConverter {
    fn flip(&self, mut p: LogicalPosition<Real>) -> LogicalPosition<Real> {
        p.y = self.monitor_height - p.y;
        p
    }

    fn to_physics_point(&self, p: LogicalPosition<Real>) -> Vect {
        self.to_physics_vec(<[Real; 2]>::from(self.flip(p)).into())
            .into()
    }

    fn to_physics_vec(&self, p: LogicalSize<Real>) -> Vect {
        Vect::from(<[_; 2]>::from(p))
    }

    fn to_logical_winit_position(&self, v: Vect) -> LogicalPosition<Real> {
        self.flip(<[Real; 2]>::from(self.to_logical_size(v)).into())
    }

    fn to_logical_size(&self, v: Vect) -> LogicalSize<Real> {
        <[_; 2]>::from(v).into()
    }

    fn from_bevy_winit(&self, v: Vect) -> LogicalPosition<Real> {
        self.flip(<[Real; 2]>::from(v).into())
    }
}

fn setup(
    mut commands: Commands,
    window: Query<Entity, With<Window>>,
    winit_windows: NonSend<WinitWindows>,
) {
    const WINDOW_INNER: Group = Group::GROUP_1;

    let window = window.get_single().unwrap();
    let window = winit_windows.get_window(window).unwrap();

    let monitor = window.current_monitor().unwrap();
    let monitor_height = monitor.size().to_logical(monitor.scale_factor()).height;

    let converter = CoordConverter { monitor_height };
    commands.insert_resource(converter);

    let camera = commands.spawn(Camera2dBundle::default()).id();

    // window
    let walls = commands
        .spawn((
            box_collider({
                let size = window
                    .inner_size()
                    .to_logical::<Real>(window.scale_factor());
                let size = converter.to_physics_vec(size) / 2.;
                size.into()
            }),
            Friction::new(0.8),
            Restitution::new(0.3),
            CollisionGroups::new(!WINDOW_INNER, Group::ALL),
            WindowWalls,
        ))
        .id();

    commands
        .spawn((
            RigidBody::KinematicPositionBased,
            LockedAxes::ROTATION_LOCKED,
            {
                let size = window
                    .outer_size()
                    .to_logical::<Real>(window.scale_factor());
                let halfbounds = converter.to_physics_vec(size) / 2.;
                Collider::cuboid(halfbounds[0], halfbounds[1])
            },
            TransformBundle::default(),
            ExternalImpulse::default(),
            Friction::new(0.8),
            Restitution::new(0.3),
            CollisionGroups::new(Group::ALL, WINDOW_INNER),
            WindowState::default(),
        ))
        .add_child(walls)
        .add_child(camera);

    // monitor
    let monitor_size = monitor.size().to_logical::<Real>(monitor.scale_factor());
    let monitor_size = converter.to_physics_vec(monitor_size);

    commands.spawn((
        box_collider((monitor_size / 2.).into()),
        TransformBundle::from(Transform::from_translation((monitor_size / 2.).extend(0.))),
        Friction::new(0.8),
        Restitution::new(0.3),
        CollisionGroups::new(Group::ALL, WINDOW_INNER),
    ));

    for _ in 0..10 {
        use rand::seq::SliceRandom;
        const COLOURS: &[Color] = &[
            Color::RED,
            Color::ORANGE,
            Color::PINK,
            Color::BLUE,
            Color::GOLD,
        ];

        let size = rand::random::<Real>() * 0.03 + 0.01;
        let size = size * PIXELS_PER_METER;

        enum Choice {
            Circle,
            Square,
        }

        let (path, cshape) = {
            match [Choice::Circle, Choice::Square]
                .choose(&mut rand::thread_rng())
                .unwrap()
            {
                Choice::Circle => (
                    blyon::geometry::GeometryBuilder::build_as(&blyon::shapes::Circle {
                        radius: size,
                        ..Default::default()
                    }),
                    Collider::ball(size),
                ),
                Choice::Square => (
                    blyon::geometry::GeometryBuilder::build_as(&blyon::shapes::Rectangle {
                        extents: Vec2::from([size, size]),
                        origin: blyon::shapes::RectangleOrigin::Center,
                    }),
                    Collider::cuboid(size / 2.0, size / 2.0),
                ),
            }
        };

        let fill = blyon::draw::Fill::color(
            *COLOURS
                .choose(&mut rand::thread_rng())
                .expect("COLOURS is not empty"),
        );

        commands.spawn((
            blyon::entity::ShapeBundle {
                path,
                ..Default::default()
            },
            RigidBody::default(),
            cshape,
            fill,
            Friction::new(0.3),
            Restitution::new(0.5),
            CollisionGroups::new(!WINDOW_INNER, Group::ALL),
        ));
    }
}

fn debug(shapes: Query<&Transform, With<blyon::prelude::Path>>) {
    // shapes.for_each(|s| {
    //     dbg!(s.translation);
    // });
}

fn window_background_indicates_state(
    mut background: ResMut<ClearColor>,
    window: Query<&WindowState>,
) {
    *background = match window.single() {
        WindowState::Bouncing => ClearColor(Color::NAVY),
        WindowState::Dragging(_) => ClearColor(Color::DARK_GRAY),
        WindowState::Static => ClearColor(Color::GRAY),
    }
}

fn update_physics_or_application_window(
    window: Query<Entity, With<Window>>,
    mut window_query: Query<(&WindowState, &mut Transform)>,
    winit_windows: NonSend<WinitWindows>,
    converter: Res<CoordConverter>,
) {
    let (window_state, mut window_physics) = window_query.single_mut();
    let window = window.single();
    let window = winit_windows.get_window(window).unwrap();

    let size = window
        .outer_size()
        .to_logical::<Real>(window.scale_factor());
    let size = converter.to_physics_vec(size);
    let offset = Vect::new(size[0], -size[1]) / 2.;

    match window_state {
        WindowState::Bouncing => {
            let center: Vect = window_physics.translation.truncate();

            let top_left = center - offset;

            window.set_outer_position(converter.to_logical_winit_position(top_left));
        }
        WindowState::Static => {
            let top_left = window
                .inner_position()
                .unwrap()
                .to_logical::<Real>(window.scale_factor());
            let top_left = converter.to_physics_point(top_left);

            let center = top_left + offset;

            window_physics.translation = center.extend(0.);
        }
        WindowState::Dragging(_) => {}
    }
}

fn window_physics_type_update(
    mut window_query: Query<(&WindowState, &mut RigidBody), Changed<WindowState>>,
) {
    if let Ok((window, mut rbtype)) = window_query.get_single_mut() {
        *rbtype = match window {
            WindowState::Bouncing => RigidBody::Dynamic,
            WindowState::Static | WindowState::Dragging(_) => RigidBody::KinematicPositionBased,
        }
    }
}

// this doesn't update Window, also uses internal instead of external coordinates
fn resize_update(
    mut resized_events: EventReader<WindowResized>,
    mut window_query: Query<&mut Collider, With<WindowWalls>>,
    converter: Res<CoordConverter>,
) {
    // let mut window_collider = window_query.single_mut();
    // for event in resized_events.iter() {
    //     let new_dims = converter.to_physics_vec([event.width, event.height].into());
    //     let new_dims = new_dims / 2.;
    //     *window_collider = box_collider(new_dims.into()).into();
    // }
}

fn toggle_physics_on_spacebar(keys: Res<Input<KeyCode>>, mut window: Query<&mut WindowState>) {
    if keys.just_pressed(KeyCode::Space) {
        let mut window = window.single_mut();
        *window = match *window {
            WindowState::Static | WindowState::Dragging(_) => WindowState::Bouncing,
            WindowState::Bouncing => WindowState::Static,
        }
    }
}

fn clicking_freezes_window(
    mouse_button: Res<Input<MouseButton>>,
    mut window: Query<&mut WindowState>,
    windows: Query<&Window>,
    converter: Res<CoordConverter>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let mut window_state = window.single_mut();
        let window = windows.get_single().unwrap();
        if let Some(p) = window.cursor_position() {
            *window_state = WindowState::Dragging(converter.from_bevy_winit(p));
        } else {
            debug!("Failed to get cursor for drag start")
        }
    }
}

fn dragging_flings_window(
    mouse_button: Res<Input<MouseButton>>,
    mut window_state: Query<(&mut WindowState, &mut ExternalImpulse)>,
    window: Query<&Window>,
    converter: Res<CoordConverter>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        let (mut window_state, mut impulse) = window_state.single_mut();
        let window = window.get_single().unwrap();
        if let WindowState::Dragging(prev) = *window_state {
            *window_state = WindowState::Bouncing;
            if let Some(curr) = window.cursor_position() {
                let prev = converter.to_physics_point(prev);
                let curr = converter.to_physics_point(converter.from_bevy_winit(curr));
                impulse.impulse = dbg!((curr - prev) * 2.0 * PIXELS_PER_METER.powi(3));
            } else {
                debug!("Failed to get cursor for drag end")
            }
        }
    }
}

struct WindowPhysicsPlugin;

impl Plugin for WindowPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(
            PIXELS_PER_METER,
        ))
        // .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(blyon::plugin::ShapePlugin)
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
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "window.velocity".into(),
                resolution: (600., 400.).into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(WindowPhysicsPlugin)
        .add_system(debug)
        .run();
}
