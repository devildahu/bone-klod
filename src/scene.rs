use std::{
    error::Error,
    path::{Path, PathBuf},
};

use bevy::{
    asset::AssetPath,
    ecs::{
        query::{QueryItem, ReadOnlyWorldQuery, WorldQuery, WorldQueryGats},
        system::{SystemParam, SystemState},
    },
    math::Vec3A,
    prelude::{Plugin as BevyPlugin, *},
    render::primitives::{Aabb, Sphere},
    scene::{InstanceId, SceneInstance},
    ui::FocusPolicy,
    utils::HashMap,
};
#[cfg(feature = "editor")]
use bevy_editor_pls_default_windows::hierarchy::picking::IgnoreEditorRayCast;
#[cfg(feature = "editor")]
use bevy_mod_picking::{PickableMesh, Selection};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    audio::ImpactSound,
    ball::{spawn_klod, Agglomerable, Klod, KlodSpawnTransform},
    cam::OrbitCamera,
    game_audio::{MusicTrigger, NoiseOnHit},
    powers::{ElementalObstacle, Power},
    prefabs::{AggloData, MusicTriggerData, Prefab, Scenery, SerdeCollider, SerdeTransform},
};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PhysicsObject {
    name: String,
    asset_path: Option<AssetPath<'static>>,
    transform: SerdeTransform,
    collider: SerdeCollider,
    friction: f32,
    restitution: f32,
    sounds: Vec<ImpactSound>,
    object: ObjectType,
}
#[derive(WorldQuery)]
struct ObjectQuery<Q>
where
    Q: ReadOnlyWorldQuery,
    for<'w> QueryItem<'w, Q>: Into<ObjectType>,
    for<'w> <Q as WorldQueryGats<'w>>::Fetch: Clone,
{
    name: Option<&'static Name>,
    sounds: &'static NoiseOnHit,
    scene: Option<&'static Handle<Scene>>,
    transform: &'static Transform,
    friction: &'static Friction,
    restitution: &'static Restitution,
    collider: &'static Collider,
    object: Q,
}
impl<'w, Q> ObjectQueryItem<'w, Q>
where
    Q: ReadOnlyWorldQuery,
    for<'ww> QueryItem<'ww, Q>: Into<ObjectType>,
    for<'ww> <Q as WorldQueryGats<'ww>>::Fetch: Clone,
{
    fn data(self, assets: &AssetServer) -> PhysicsObject {
        PhysicsObject {
            sounds: self.sounds.noises.to_vec(),
            asset_path: self
                .scene
                .and_then(|h| assets.get_handle_path(h))
                .map(|t| t.to_owned()),
            transform: (*self.transform).into(),
            object: self.object.into(),
            name: self
                .name
                .and_then(|name| (name.as_str() != "").then(|| name.to_string()))
                .unwrap_or_else(|| "Unamed Physics Object".to_owned()),
            collider: self.collider.into(),
            friction: self.friction.coefficient,
            restitution: self.restitution.coefficient,
        }
    }
}

impl PhysicsObject {
    pub(crate) fn new(
        name: String,
        asset_path: String,
        transform: Transform,
        collider: SerdeCollider,
        friction: f32,
        restitution: f32,
        sounds: Vec<ImpactSound>,
        object: ObjectType,
    ) -> Self {
        Self {
            name,
            sounds,
            asset_path: Some(AssetPath::from(&asset_path).to_owned()),
            transform: transform.into(),
            object,
            collider,
            friction,
            restitution,
        }
    }

    pub(crate) fn spawn(
        self,
        cmds: &mut Commands,
        assets: &AssetServer,
        meshes: &mut Assets<Mesh>,
        compute_aabb: bool,
    ) {
        let asset_path = match self.asset_path {
            Some(path) => path,
            None => return,
        };
        let mut object = cmds.spawn_bundle(SceneBundle {
            scene: assets.load(asset_path),
            transform: self.transform.into(),
            ..default()
        });
        object.insert_bundle((
            Name::new(self.name),
            NoiseOnHit { noises: self.sounds.iter().cloned().collect() },
            Collider::from(self.collider.clone()),
            Friction {
                coefficient: self.friction,
                combine_rule: CoefficientCombineRule::Max,
            },
            Restitution {
                coefficient: self.restitution,
                combine_rule: CoefficientCombineRule::Max,
            },
        ));
        if compute_aabb {
            object.insert(ComputeDefaultAabb);
        }
        #[cfg(feature = "editor")]
        object.insert_bundle((
            meshes.add(self.collider.into()),
            bevy_scene_hook::SceneHook::new(|_, cmds| {
                cmds.insert(IgnoreEditorRayCast);
            }),
            PickableMesh::default(),
            Interaction::default(),
            FocusPolicy::default(),
            Selection::default(),
            bevy_transform_gizmo::GizmoTransformable,
        ));
        match self.object {
            ObjectType::Scenery(scenery_data) => scenery_data.spawn(&mut object),
            ObjectType::Agglomerable(agglo_data) => agglo_data.spawn(&mut object),
        };
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub(crate) enum ObjectType {
    Scenery(Scenery),
    Agglomerable(AggloData),
}
impl<'w> From<(&'w Scenery, Option<&'w ElementalObstacle>)> for ObjectType {
    fn from(item: QueryItem<'w, <Scenery as Prefab>::Query>) -> Self {
        ObjectType::Scenery(Prefab::from_query(item))
    }
}

impl<'w> From<(&'w Agglomerable, &'w Power)> for ObjectType {
    fn from(item: QueryItem<'w, <AggloData as Prefab>::Query>) -> Self {
        ObjectType::Agglomerable(Prefab::from_query(item))
    }
}

#[derive(SystemParam)]
struct KlodSceneQuery<'w, 's> {
    assets: Res<'w, AssetServer>,
    agglomerables: Query<'w, 's, ObjectQuery<<AggloData as Prefab>::Query>>,
    scenery: Query<'w, 's, ObjectQuery<<Scenery as Prefab>::Query>>,
    music: Query<'w, 's, <MusicTriggerData as Prefab>::Query>,
    klod_spawn: Res<'w, KlodSpawnTransform>,
}
#[derive(SystemParam)]
struct KlodSweepQuery<'w, 's> {
    query: Query<'w, 's, Entity, Or<(With<Scenery>, With<Agglomerable>, With<MusicTrigger>)>>,
}
impl<'w, 's> KlodSweepQuery<'w, 's> {
    pub(crate) fn to_sweep(&self) -> Vec<Entity> {
        self.query.iter().collect()
    }
}
#[derive(SystemParam)]
struct KlodSpawnQuery<'w, 's> {
    cmds: Commands<'w, 's>,
    assets: Res<'w, AssetServer>,
    meshes: ResMut<'w, Assets<Mesh>>,
    klod_spawn: ResMut<'w, KlodSpawnTransform>,
    klod: Query<'w, 's, Entity, With<Klod>>,
}
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct KlodScene {
    klod_spawn_transform: SerdeTransform,
    objects: Vec<PhysicsObject>,
    music_triggers: Vec<MusicTriggerData>,
}
#[derive(SystemParam)]
struct KlodCopyQuery<'w, 's> {
    cmds: Commands<'w, 's>,
    assets: Res<'w, AssetServer>,
    meshes: ResMut<'w, Assets<Mesh>>,
    agglomerables: Query<'w, 's, ObjectQuery<<AggloData as Prefab>::Query>>,
    scenery: Query<'w, 's, ObjectQuery<<Scenery as Prefab>::Query>>,
}
impl KlodScene {
    pub(crate) fn copy_objects(objects: &[Entity], world: &mut World) {
        let mut query = SystemState::<KlodCopyQuery>::new(world);
        let KlodCopyQuery {
            agglomerables,
            scenery,
            assets,
            mut cmds,
            mut meshes,
        } = query.get_mut(world);
        let o = objects;
        let mut to_copy = Vec::new();
        to_copy.extend(agglomerables.iter_many(o).map(|item| item.data(&assets)));
        to_copy.extend(scenery.iter_many(o).map(|item| item.data(&assets)));

        for mut object in to_copy.into_iter() {
            object.name = format!("Copy of {}", object.name);
            object.spawn(&mut cmds, &assets, &mut meshes, false);
        }
        query.apply(world);
    }
    fn spawn(self, KlodSpawnQuery { cmds, assets, meshes, klod_spawn, klod }: &mut KlodSpawnQuery) {
        klod_spawn.0 = self.klod_spawn_transform.into();

        let klod = if let Ok(klod) = klod.get_single() {
            klod
        } else {
            let klod = spawn_klod(cmds, assets);
            cmds.spawn_bundle(Camera3dBundle {
                transform: Transform::from_xyz(-10.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            })
            .insert_bundle((OrbitCamera::follows(klod), Name::new("Klod Camera")));
            klod
        };
        cmds.entity(klod).insert(klod_spawn.0);

        for object in self.objects.into_iter() {
            object.spawn(cmds, assets, meshes, false);
        }
        for music in self.music_triggers.into_iter() {
            let mut cmds = cmds.spawn();
            #[cfg(feature = "editor")]
            cmds.insert(meshes.add(music.collider.clone().into()));
            music.spawn(&mut cmds);
        }
    }
    fn read(
        KlodSceneQuery { assets, agglomerables, scenery, klod_spawn, music }: &KlodSceneQuery,
    ) -> Self {
        let mut objects = Vec::with_capacity(agglomerables.iter().len() + scenery.iter().len());
        objects.extend(agglomerables.iter().map(|item| item.data(assets)));
        objects.extend(scenery.iter().map(|item| item.data(assets)));
        let music_triggers = music.iter().map(|t| Prefab::from_query(t)).collect();
        KlodScene {
            objects,
            klod_spawn_transform: klod_spawn.0.into(),
            music_triggers,
        }
    }

    pub(crate) fn load(
        world: &mut World,
        scene_path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let file = std::fs::File::open(scene_path)?;
        let mut system_state = SystemState::<KlodSweepQuery>::new(world);
        let to_sweep = system_state.get(world).to_sweep();
        for entity in to_sweep.into_iter() {
            world.entity_mut(entity).despawn_recursive();
        }
        let mut system_state = SystemState::<KlodSpawnQuery>::new(world);
        let scene: KlodScene = ron::de::from_reader(file)?;
        let mut query = system_state.get_mut(world);
        scene.spawn(&mut query);
        system_state.apply(world);
        Ok(())
    }
    pub(crate) fn save(
        world: &mut World,
        scene_path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut system_state = SystemState::<KlodSceneQuery>::new(world);
        let scene = KlodScene::read(&system_state.get_mut(world));
        let serialized = ron::ser::to_string_pretty(
            &scene,
            ron::ser::PrettyConfig::new()
                .indentor(" ".to_owned())
                .depth_limit(80),
        )?;
        std::fs::write(scene_path, serialized)?;
        Ok(())
    }
}

#[derive(Component)]
struct ComputeDefaultAabb;

fn add_scene_aabb(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    scene_instances: Query<
        (Entity, &SceneInstance, &Transform),
        (Added<SceneInstance>, With<ComputeDefaultAabb>),
    >,
    scenes: Res<SceneSpawner>,
    mut to_visit: Local<HashMap<Entity, (InstanceId, Vec3A)>>,
    meshes: Query<(&GlobalTransform, &Aabb), With<Handle<Mesh>>>,
) {
    for (entity, instance, transform) in &scene_instances {
        to_visit.insert(entity, (**instance, transform.scale.into()));
        commands.entity(entity).remove::<ComputeDefaultAabb>();
    }
    let mut visited = Vec::new();
    for (entity, (to_visit, scale)) in to_visit.iter() {
        let entities = match scenes.iter_instance_entities(*to_visit) {
            Some(entities) if scenes.instance_is_ready(*to_visit) => entities,
            _ => continue,
        };
        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        for entity in entities {
            if let Ok((transform, aabb)) = meshes.get(entity) {
                // If the Aabb had not been rotated, applying the non-uniform scale would produce the
                // correct bounds. However, it could very well be rotated and so we first convert to
                // a Sphere, and then back to an Aabb to find the conservative min and max points.
                let sphere = Sphere {
                    center: Vec3A::from(transform.mul_vec3(Vec3::from(aabb.center))),
                    radius: transform.radius_vec3a(aabb.half_extents),
                };
                let aabb = Aabb::from(sphere);
                min = min.min(aabb.min());
                max = max.max(aabb.max());
            }
        }
        let aabb = Aabb::from_min_max(Vec3::from(min), Vec3::from(max));
        visited.push((*entity, (aabb, *scale)));
    }
    for (entity, (aabb, scale)) in visited.into_iter() {
        let extents = aabb.half_extents / scale;
        let collider = SerdeCollider::Cuboid { half_extents: extents.into() };
        if aabb.min().min_element() != f32::MIN && aabb.max().max_element() != f32::MAX {
            commands.entity(entity).insert_bundle((
                Collider::from(collider.clone()),
                mesh_assets.add(collider.into()),
                aabb,
            ));
        }
        to_visit.remove(&entity);
    }
}

fn fit_pickbox_to_collider(
    mut colliders: Query<(&Collider, &Handle<Mesh>, &mut Aabb), Changed<Collider>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (collider, mesh, mut aabb) in &mut colliders {
        if let Some(mesh) = meshes.get_mut(mesh) {
            *mesh = SerdeCollider::from(collider).into();
            if let Some(new_aabb) = mesh.compute_aabb() {
                *aabb = new_aabb;
            }
        }
    }
}

/// Returns the base path of the assets directory, which is normally the executable's parent
/// directory.
///
/// If the `CARGO_MANIFEST_DIR` environment variable is set, then its value will be used
/// instead. It's set by cargo when running with `cargo run`.
pub(crate) fn get_base_path() -> PathBuf {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir)
    } else {
        let run = || Some(std::env::current_exe().ok()?.parent()?.to_owned());
        run().unwrap()
    }
    .join("assets")
}

pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PostUpdate, add_scene_aabb)
            .add_system(fit_pickbox_to_collider);
    }
}
