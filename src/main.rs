use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_mod_picking::{prelude::*, backend::{HitData, PointerHits}, picking_core::PickSet};

fn main() {
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultPickingPlugins)
        .add_systems(Startup, setup)

        .add_systems(PreUpdate, circle_picking_backend.in_set(PickSet::Backend))
        .add_systems(Update, draw_circles)
    ;
    app.run();
}

#[derive(Component)]
struct Circle {
    radius: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    server: Res<AssetServer>,
){
    commands.spawn(
        Camera2dBundle::default()
    );

    // Mesh circle with default picking backend in front
    commands.spawn((
        MaterialMesh2dBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
            mesh: meshes.add(shape::Circle::new(50.0).into()).into(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..default()
        },
        PickableBundle::default(),
    ));
    // Circle with custom picking backend behind 
    commands.spawn((
        Transform::from_translation(Vec3::new(0., 50., -0.1)),
        Circle {
            radius: 50.0,
        },
        PickableBundle::default(),
    ));
}

// Simple system to draw the circles
fn draw_circles(
    circles: Query<(&Transform, &Circle)>,
    mut gizmos: Gizmos,
){
    for (tr, circ) in circles.iter() {
        gizmos.circle_2d(tr.translation.truncate(), circ.radius, Color::RED);
    }
}

// Custom picking backend. The backend in my actual repository is almost the same, only it calculates a distance to a line instead of a circle.
fn circle_picking_backend(
    pointers: Query<(&PointerId, &PointerLocation)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform)>,
    primary_window: Query<Entity, With<bevy::window::PrimaryWindow>>,

    edges: Query<(
        Entity,
        &Transform,
        &Circle,
        Option<&Pickable>,
    )>,

    mut output: EventWriter<backend::PointerHits>,
){
    
    for (pointer, location) in pointers.iter().filter_map(|(pointer, pointer_location)| {
        pointer_location.location().map(|loc| (pointer, loc))
    }) {
        let Some((cam_entity, camera, cam_transform)) = cameras
            .iter()
            .filter(|(_, camera, _)| camera.is_active)
            .find(|(_, camera, _)| {
                camera
                    .target
                    .normalize(Some(primary_window.single()))
                    .unwrap()
                    == location.target
            })
        else {
            continue;
        };

        let Some(cursor_pos_world) = camera.viewport_to_world_2d(cam_transform, location.position)
        else {
            continue;
        };

        let mut picks_presort: Vec<(Entity, f32, f32)> = edges
            .iter()
            .filter_map(|(entity, transform, circle, _, ..)| {
                // Calculate the distance from the pointer to the circle 
                let threshold = circle.radius;             
                let distance = cursor_pos_world.distance(transform.translation.truncate());
                let within_bounds = distance < threshold;
                
                within_bounds.then_some((
                    entity,
                    distance,
                    transform.translation.z,
                ))

            })
            .collect();


        // Sort the picks by distance
        picks_presort.sort_by(|(_, adist, _), (_, bdist, _)| adist.partial_cmp(&bdist).unwrap());

        let picks_sort: Vec<(Entity, HitData)> = picks_presort.iter().map(|(entity, dist, z)| {
            // println!("Edge z: {:?}", z);
            (*entity, HitData::new(cam_entity, *z, None, None))
        })
        .collect();

        let order = camera.order as f32;
        output.send(PointerHits::new(*pointer, picks_sort, order))
    }
}