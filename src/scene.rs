use std::{error::Error, path::Path};

use bevy::{
    asset::AssetPath,
    ecs::{
        query::{QueryItem, ReadOnlyWorldQuery, WorldQuery, WorldQueryGats},
        system::{SystemParam, SystemState},
    },
    prelude::*,
    ui::FocusPolicy,
};
use bevy_editor_pls_default_windows::hierarchy::picking::IgnoreEditorRayCast;
use bevy_mod_picking::{PickableMesh, Selection};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ball::{Agglomerable, Scenery},
    prefabs::{AggloData, Prefab, SceneryData},
};

#[derive(Serialize, Debug, Deserialize)]
struct SerdeTransform {
    rotation: Quat,
    scale: Vec3,
    translation: Vec3,
}
impl From<Transform> for SerdeTransform {
    fn from(item: Transform) -> Self {
        SerdeTransform {
            rotation: item.rotation,
            scale: item.scale,
            translation: item.translation,
        }
    }
}
impl From<SerdeTransform> for Transform {
    fn from(item: SerdeTransform) -> Self {
        Transform {
            rotation: item.rotation,
            scale: item.scale,
            translation: item.translation,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PhysicsObject {
    name: String,
    asset_path: Option<AssetPath<'static>>,
    transform: SerdeTransform,
    object: ObjectType,
}
#[derive(WorldQuery)]
struct ObjectQuery<Q>
where
    Q: ReadOnlyWorldQuery,
    for<'a, 'w> &'a QueryItem<'w, Q>: Into<ObjectType>,
    for<'w> <Q as WorldQueryGats<'w>>::Fetch: Clone,
{
    name: Option<&'static Name>,
    scene: &'static Handle<Scene>,
    transform: &'static Transform,
    object: Q,
}
impl<'w, Q> ObjectQueryItem<'w, Q>
where
    Q: ReadOnlyWorldQuery,
    for<'a, 'ww> &'a QueryItem<'ww, Q>: Into<ObjectType>,
    for<'ww> <Q as WorldQueryGats<'ww>>::Fetch: Clone,
{
    fn data(&self, assets: &AssetServer) -> PhysicsObject {
        PhysicsObject {
            asset_path: assets.get_handle_path(self.scene).map(|t| t.to_owned()),
            transform: (*self.transform).into(),
            object: (&self.object).into(),
            name: self
                .name
                .map_or_else(|| "Unamed Physics Object".to_owned(), |n| n.to_string()),
        }
    }
}

impl PhysicsObject {
    pub(crate) fn new(
        name: String,
        asset_path: String,
        transform: Transform,
        object: ObjectType,
    ) -> Self {
        Self {
            name,
            asset_path: Some(AssetPath::new(asset_path.into(), Some("Scene0".to_owned()))),
            transform: transform.into(),
            object,
        }
    }

    pub(crate) fn spawn(
        self,
        cmds: &mut Commands,
        assets: &AssetServer,
        meshes: &mut Assets<Mesh>,
    ) {
        let asset_path = match self.asset_path {
            Some(path) => path,
            None => return,
        };
        let mut object = cmds.spawn_bundle(SceneBundle {
            scene: dbg!(assets.load(asset_path)),
            transform: self.transform.into(),
            ..default()
        });
        object.insert(Name::new(self.name));
        #[cfg(feature = "editor")]
        object.insert_bundle((
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
            ObjectType::Scenery(scenery_data) => scenery_data.spawn(&mut object, meshes),
            ObjectType::Agglomerable(agglo_data) => agglo_data.spawn(&mut object, meshes),
        };
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub(crate) enum ObjectType {
    Scenery(SceneryData),
    Agglomerable(AggloData),
}
impl<'a, 'w> From<&'a (&'w Scenery, &'w Collider, &'w Friction, &'w Restitution)> for ObjectType {
    fn from(item: &'a QueryItem<'w, <SceneryData as Prefab>::Query>) -> Self {
        ObjectType::Scenery(Prefab::from_query(item))
    }
}

impl<'a, 'w>
    From<&'a (
        &'w Collider,
        &'w Agglomerable,
        &'w Friction,
        &'w Restitution,
    )> for ObjectType
{
    fn from(item: &'a QueryItem<'w, <AggloData as Prefab>::Query>) -> Self {
        ObjectType::Agglomerable(Prefab::from_query(item))
    }
}

#[derive(SystemParam)]
struct KlodSceneQuery<'w, 's> {
    assets: Res<'w, AssetServer>,
    agglomerables: Query<'w, 's, ObjectQuery<<AggloData as Prefab>::Query>>,
    scenery: Query<'w, 's, ObjectQuery<<SceneryData as Prefab>::Query>>,
}
#[derive(SystemParam)]
struct KlodSweepQuery<'w, 's> {
    query: Query<'w, 's, Entity, Or<(With<Scenery>, With<Agglomerable>)>>,
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
}
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct KlodScene(Vec<PhysicsObject>);
impl KlodScene {
    fn spawn(self, KlodSpawnQuery { cmds, assets, meshes }: &mut KlodSpawnQuery) {
        println!("Adding back entities from serialized scene: {:?}", &self);
        for object in self.0.into_iter() {
            object.spawn(cmds, assets, meshes);
        }
    }
    fn read(KlodSceneQuery { assets, agglomerables, scenery }: &KlodSceneQuery) -> Self {
        let mut scene = Vec::with_capacity(agglomerables.iter().len() + scenery.iter().len());
        scene.extend(agglomerables.iter().map(|item| item.data(assets)));
        scene.extend(scenery.iter().map(|item| item.data(assets)));
        KlodScene(scene)
    }

    pub(crate) fn load(
        world: &mut World,
        scene_path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut system_state = SystemState::<KlodSweepQuery>::new(world);
        let to_sweep = system_state.get(world).to_sweep();
        for entity in to_sweep.into_iter() {
            world.entity_mut(entity).despawn_recursive();
        }
        let mut system_state = SystemState::<KlodSpawnQuery>::new(world);
        let file = std::fs::read_to_string(scene_path)?;
        let scene: KlodScene = ron::from_str(&file)?;
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
/*
// TODO: compute full scene AABB (probably not enough time for this jam)
fn scene_aabb(
    mut commands: Commands,
    scene_instances: Query<(Entity, &SceneInstance), Added<SceneInstance>>,
    scenes: Res<SceneSpawner>,
    mut to_visit: Local<HashSet<(Entity, InstanceId)>>,
    meshes: Query<(&GlobalTransform, &Aabb), With<Handle<Mesh>>>,
) {
    for (entity, instance) in &scene_instances {
        to_visit.insert((entity, **instance));
    }
    let mut visited = Vec::new();
    for (entity, to_visit) in to_visit.iter() {
        if !scenes.instance_is_ready(*to_visit) {
            continue;
        }
        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        for entity in scenes.iter_instance_entities(*to_visit) {
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
        visited.push((*to_visit, aabb));
    }
    for (entity, visited) in visited.into_iter() {
        commands.entity(entity).insert(aabb);
        to_visit.remove(&visited);
    }
}
*/
