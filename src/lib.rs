use std::fmt::Debug;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

pub use bevy_replicon_snap_macros;

use crate::{
    interpolation::{Interpolated, SnapshotInterpolationConfig},
    prediction::{owner_prediction_init_system, OwnerPredicted, Predicted},
};

pub mod interpolation;
pub mod prediction;

pub struct SnapshotInterpolationPlugin {
    /// Should reflect the server max tick rate
    pub max_tick_rate: u16,
}

#[derive(Component, Deserialize, Serialize, Reflect)]
pub struct NetworkOwner(pub u64);

/// Sets for interpolation systems.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum InterpolationSet {
    /// Systems that initializes buffers and flag components for replicated entities.
    ///
    /// Runs in `PreUpdate`.
    Init,
    /// Systems that calculating interpolation.
    ///
    /// Runs in `PreUpdate`.
    Interpolate,
}

impl Plugin for SnapshotInterpolationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Interpolated>()
            .register_type::<OwnerPredicted>()
            .register_type::<NetworkOwner>()
            .register_type::<Predicted>()
            .replicate::<Interpolated>()
            .replicate::<NetworkOwner>()
            .replicate::<OwnerPredicted>()
            .configure_sets(PreUpdate, InterpolationSet::Init.after(ClientSet::Receive))
            .configure_sets(
                PreUpdate,
                InterpolationSet::Interpolate.after(InterpolationSet::Init),
            )
            .add_systems(
                Update,
                owner_prediction_init_system
                    .run_if(client_connected)
                    .in_set(InterpolationSet::Init),
            )
            .insert_resource(SnapshotInterpolationConfig {
                max_tick_rate: self.max_tick_rate,
            });
    }
}

#[proc_macro_derive(Interpolate)]
pub fn derive_interpolate(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let body = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let field_name = fields.named.iter().map(|field| &field.ident);
            quote! {
                Self {
                    #(
                        #field_name: self.#field_name.lerp(other.value, t),
                    )*
                }
            }
        }
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        }) => quote! { Self(self.0.lerp(other.0, t)) },
        _ => panic!("expected a struct"),
    };
    let output = quote! {
        impl bevy_replicon_snap::interpolation::Interpolate for #ident {
            fn interpolate(&self, other: Self, t: f32) -> Self {
              #body
            }
        }
    };
    output.into()
}