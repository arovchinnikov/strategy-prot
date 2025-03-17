use bevy::app::{Startup, Update};
use bevy::input::mouse::MouseWheel;
use bevy::math::Vec3;
use bevy::prelude::{ButtonInput, Camera, Camera3d, Commands, Component, EulerRot, EventReader, FixedUpdate, GlobalTransform, KeyCode, MouseButton, Quat, Query, Ray3d, Res, ResMut, Resource, Time, Transform, Vec2, Window};

const MAP_MIN_X: f32 = -256.0;
const MAP_MAX_X: f32 = 8192.0 + 256.0;
const MAP_MIN_Z: f32 = -256.0;
const MAP_MAX_Z: f32 = 4096.0 + 256.0;

#[derive(Component)]
struct CameraController {
    zoom: CameraZoom
}

struct CameraZoom {
    speed: f32,
    target_height: f32,
    current_height: f32,
    smooth_factor: f32,
}

#[derive(Resource, Default)]
struct CameraDragState {
    is_dragging: bool,
    drag_start_world_position: Option<Vec3>,
}

pub fn build(app: &mut bevy::prelude::App) {
    app.add_systems(Startup, spawn_camera);
    app.add_systems(Update, camera_drag_movement);
    app.add_systems(FixedUpdate, zoom_handler);
    app.add_systems(FixedUpdate, camera_movement);
    app.init_resource::<CameraDragState>();
}

fn spawn_camera(mut commands: Commands) {
    let initial_height = 120.0;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1100.0, initial_height, 720.0),
        CameraController {
            zoom: CameraZoom {
                speed: 1200.0,
                target_height: initial_height,
                current_height: initial_height,
                smooth_factor: 0.1,
            }
        }
    ));
}

fn camera_drag_movement(
    window: Query<&Window>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<CameraDragState>,
    mut query: Query<(&mut Transform, &GlobalTransform, &Camera)>,
) {
    let window = window.single();
    let (mut transform, global_transform, camera) = query.single_mut();

    if mouse_input.just_pressed(MouseButton::Right) {
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok(ray) = camera.viewport_to_world(global_transform, cursor_position) {
                if let Some(world_position) = ray_intersect_plane(ray, Vec3::Y, 0.0) {
                    drag_state.is_dragging = true;
                    drag_state.drag_start_world_position = Some(world_position);
                }
            }
        }
    }

    if mouse_input.just_released(MouseButton::Right) {
        drag_state.is_dragging = false;
        drag_state.drag_start_world_position = None;
    }

    if drag_state.is_dragging {
        if let Some(cursor_position) = window.cursor_position() {
            if let Some(start_world_pos) = drag_state.drag_start_world_position {
                if let Ok(ray) = camera.viewport_to_world(global_transform, cursor_position) {
                    if let Some(current_world_pos) = ray_intersect_plane(ray, Vec3::Y, 0.0) {
                        let world_delta = start_world_pos - current_world_pos;

                        let movement = Vec3::new(world_delta.x, 0.0, world_delta.z);
                        let new_position = transform.translation + movement;
                        transform.translation = clamp_camera_position(new_position);
                    }
                }
            }
        }
    }
}

fn ray_intersect_plane(ray: Ray3d, plane_normal: Vec3, plane_d: f32) -> Option<Vec3> {
    let denom = ray.direction.dot(plane_normal);
    if denom.abs() > f32::EPSILON {
        let t = -(ray.origin.dot(plane_normal) + plane_d) / denom;
        if t >= 0.0 {
            return Some(ray.origin + t * ray.direction);
        }
    }
    None
}

fn clamp_camera_position(position: Vec3) -> Vec3 {
    Vec3::new(
        position.x.clamp(MAP_MIN_X, MAP_MAX_X),
        position.y,
        position.z.clamp(MAP_MIN_Z, MAP_MAX_Z),
    )
}

const MIN_HEIGHT: f32 = 60.0;
const MAX_HEIGHT: f32 = 1300.0;
const MIN_TILT: f32 = -0.6;
const MAX_TILT: f32 = -1.35;

pub fn zoom_handler(
    time: Res<Time>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query: Query<(&mut CameraController, &mut Transform)>,
) {
    let mut scroll = 0.0;
    for event in mouse_wheel_events.read() {
        scroll -= event.y;
    }

    for (mut controller, mut transform) in query.iter_mut() {
        controller.zoom.target_height -= scroll * controller.zoom.speed * time.delta_secs();
        controller.zoom.target_height = controller.zoom.target_height.clamp(MIN_HEIGHT, MAX_HEIGHT);

        if controller.zoom.target_height == controller.zoom.current_height {
            continue;
        }

        controller.zoom.current_height = lerp(
            controller.zoom.current_height,
            controller.zoom.target_height,
            controller.zoom.smooth_factor
        );

        transform.translation.y = controller.zoom.current_height;
        let pitch_angle = height_to_tilt(controller.zoom.current_height);
        let (yaw, _, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch_angle, roll);
    }
}


fn height_to_tilt(height: f32) -> f32 {
    let clamped_height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    let t = (clamped_height - MIN_HEIGHT) / (MAX_HEIGHT - MIN_HEIGHT);
    let transformed_t = 2.0 * t - t * t;
    MIN_TILT + (MAX_TILT - MIN_TILT) * transformed_t
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&CameraController, &mut Transform)>,
) {
    for (controller, mut transform) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) {
            direction.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
        }

        if direction != Vec3::ZERO {
            direction = direction.normalize();
            let new_position = transform.translation + direction * 440.0 * time.delta_secs();
            transform.translation = clamp_camera_position(new_position);
        }
    }
}
