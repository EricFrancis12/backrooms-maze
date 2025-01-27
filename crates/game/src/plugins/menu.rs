use bevy::{prelude::*, ui::RelativeCursorPosition};
use dungeon_maze_common::{
    cursor::{CursorFollower, CursorPosition},
    inventory::{
        equipment::EquipmentSlotName, item::ItemName, Inventory, InventoryChanged, ItemUsed,
    },
    menu::*,
    player::{
        DmgType, HealHealth, HealStamina, Health, Player, PlayerState, Regenerator, Stamina,
        TakeDamage,
    },
    settings::{ChunkRenderDist, GameSettings, RenderDistChanged},
    should_not_happen,
    utils::entity::get_n_parent,
};
use strum::IntoEnumIterator;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MenuOpen>()
            .init_state::<ActiveMenuTab>()
            .init_state::<DragState>()
            .add_systems(
                Update,
                (
                    toggle_menu_open,
                    change_active_menu_tab,
                    manage_menu_content,
                    update_inventory_menu_content,
                    change_menu_tabs_background_color,
                    change_render_dist,
                    change_render_dist_buttons_background_color,
                    update_visible_on_parent_hover,
                    use_inventory_item,
                    handle_item_used,
                    update_item_image_cursor_follower,
                ),
            )
            .add_systems(
                Update,
                (
                    start_drag_inventory_item,
                    start_drag_equipment_item,
                    stop_drag_item,
                )
                    .run_if(in_state(PlayerState::Walking)),
            )
            .add_systems(OnEnter(MenuOpen(true)), spawn_menu)
            .add_systems(OnExit(MenuOpen(true)), despawn_menu);
    }
}

fn toggle_menu_open(
    keys: Res<ButtonInput<KeyCode>>,
    menu_open: Res<State<MenuOpen>>,
    mut next_menu_open: ResMut<NextState<MenuOpen>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        next_menu_open.set(MenuOpen(!menu_open.0));
    }
}

fn spawn_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    inventory: Res<Inventory>,
    active_menu_tab: Res<State<ActiveMenuTab>>,
    game_settings: Res<State<GameSettings>>,
) {
    commands
        .spawn((
            Menu,
            RelativeCursorPosition::default(),
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    height: Val::Percent(80.0),
                    width: Val::Percent(20.0),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                background_color: Color::BLACK.into(),
                ..default()
            },
            Name::new("Menu"),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    MenuContent,
                    NodeBundle {
                        style: Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            height: Val::Percent(94.0),
                            width: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: Color::linear_rgba(0.0, 0.0, 0.7, 1.0).into(),
                        ..default()
                    },
                ))
                .with_children(|grandparent| match active_menu_tab.get().0 {
                    MenuTab::Inventory => {
                        spawn_inventory_menu_content(grandparent, &asset_server, &inventory)
                    }
                    MenuTab::Settings => spawn_settings_menu_content(grandparent, &game_settings),
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        height: Val::Percent(6.0),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: Color::linear_rgba(0.0, 0.7, 0.0, 1.0).into(),
                    ..default()
                })
                .with_children(|grandparent| {
                    for tab in [MenuTab::Inventory, MenuTab::Settings] {
                        grandparent.spawn((
                            ButtonBundle {
                                style: Style {
                                    height: Val::Percent(100.0),
                                    width: Val::Percent(20.0),
                                    ..default()
                                },
                                background_color: get_tab_background_color(
                                    &tab,
                                    active_menu_tab.get(),
                                ),
                                ..default()
                            },
                            Name::new(format!("Menu Tab {}", tab)),
                            tab,
                        ));
                    }
                });
        });
}

fn despawn_menu(mut commands: Commands, menu_query: Query<Entity, With<Menu>>) {
    let menu_entity = menu_query.get_single().unwrap();
    commands.entity(menu_entity).despawn_recursive();
}

fn change_active_menu_tab(
    menu_tab_query: Query<(&MenuTab, &Interaction)>,
    active_menu_tab: Res<State<ActiveMenuTab>>,
    mut next_active_menu_tab: ResMut<NextState<ActiveMenuTab>>,
) {
    for (menu_tab, interaction) in menu_tab_query.iter() {
        if *interaction == Interaction::Pressed {
            if *menu_tab != active_menu_tab.get().0 {
                next_active_menu_tab.set(ActiveMenuTab(menu_tab.clone()));
                break;
            }
        }
    }
}

fn manage_menu_content(
    mut commands: Commands,
    mut event_reader: EventReader<StateTransitionEvent<ActiveMenuTab>>,
    menu_content_query: Query<Entity, With<MenuContent>>,
    asset_server: Res<AssetServer>,
    inventory: Res<Inventory>,
    active_menu_tab: Res<State<ActiveMenuTab>>,
    game_settings: Res<State<GameSettings>>,
) {
    for _ in event_reader.read() {
        if let Ok(entity) = menu_content_query.get_single() {
            let mut entity_commands = commands.entity(entity);
            entity_commands.despawn_descendants();

            entity_commands.with_children(|parent| match active_menu_tab.get().0 {
                MenuTab::Inventory => {
                    spawn_inventory_menu_content(parent, &asset_server, &inventory);
                }
                MenuTab::Settings => spawn_settings_menu_content(parent, &game_settings),
            });
        }
    }
}

fn update_inventory_menu_content(
    mut commands: Commands,
    mut event_reader: EventReader<InventoryChanged>,
    menu_content_query: Query<Entity, With<MenuContent>>,
    asset_server: Res<AssetServer>,
    inventory: Res<Inventory>,
) {
    for _ in event_reader.read() {
        if let Ok(entity) = menu_content_query.get_single() {
            let mut entity_commands = commands.entity(entity);
            entity_commands.despawn_descendants();
            entity_commands.with_children(|parent| {
                spawn_inventory_menu_content(parent, &asset_server, &inventory);
            });
        }
    }
}

fn spawn_inventory_menu_content(
    child_builder: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    inventory: &Res<Inventory>,
) {
    child_builder.spawn(TextBundle {
        text: Text {
            sections: vec![TextSection::new(
                "Inventory",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            )],
            ..default()
        },
        ..default()
    });

    // Inventory slots
    child_builder
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for (i, slot) in inventory.slots.iter().enumerate() {
                let mut entity_commands = parent.spawn((
                    InventorySlot(i),
                    RelativeCursorPosition::default(),
                    ButtonBundle {
                        style: Style {
                            position_type: PositionType::Relative,
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            height: Val::Px(50.0),
                            width: Val::Px(50.0),
                            margin: UiRect::all(Val::Px(5.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        border_color: Color::WHITE.into(),
                        ..default()
                    },
                    Name::new(format!("Inventory Slot {}", i)),
                ));

                if let Some(item) = slot {
                    entity_commands.with_children(|grandparent| {
                        grandparent.spawn((
                            RelativeCursorPosition::default(),
                            ImageBundle {
                                image: item.ui_image(asset_server),
                                style: item_style(),
                                ..default()
                            },
                        ));

                        grandparent.spawn((
                            VisibleOnParentHover::default(),
                            TextBundle {
                                visibility: Visibility::Hidden,
                                text: Text {
                                    sections: vec![TextSection::new(
                                        item.name.to_string(),
                                        TextStyle {
                                            font_size: 22.0,
                                            color: Color::WHITE,
                                            ..default()
                                        },
                                    )],
                                    ..default()
                                },
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    top: Val::Percent(100.0),
                                    right: Val::Percent(0.0),
                                    ..default()
                                },
                                background_color: Color::BLACK.into(),
                                z_index: ZIndex::Global(10),
                                ..default()
                            },
                        ));

                        if item.amt > 1 {
                            grandparent.spawn(TextBundle {
                                text: Text {
                                    sections: vec![TextSection::new(
                                        item.amt.to_string(),
                                        TextStyle {
                                            font_size: 22.0,
                                            color: Color::WHITE,
                                            ..default()
                                        },
                                    )],
                                    ..default()
                                },
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    bottom: Val::Px(2.0),
                                    right: Val::Px(2.0),
                                    ..default()
                                },
                                background_color: Color::BLACK.into(),
                                ..default()
                            });
                        }
                    });
                }
            }
        });

    child_builder.spawn(TextBundle {
        text: Text {
            sections: vec![TextSection::new(
                "Equipment",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            )],
            ..default()
        },
        ..default()
    });

    // Equipment slots
    child_builder
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for name in EquipmentSlotName::iter() {
                let mut entity_commands = parent.spawn((
                    EquipmentSlot(name.clone()),
                    RelativeCursorPosition::default(),
                    ButtonBundle {
                        style: Style {
                            position_type: PositionType::Relative,
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            height: Val::Px(50.0),
                            width: Val::Px(50.0),
                            margin: UiRect::all(Val::Px(5.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        border_color: Color::WHITE.into(),
                        ..default()
                    },
                    Name::new(format!("Equipment slot {}", name)),
                ));

                if let Some(item) = inventory.equipment.at(&name) {
                    entity_commands.with_children(|grandparent| {
                        grandparent.spawn((
                            RelativeCursorPosition::default(),
                            ImageBundle {
                                image: item.ui_image(asset_server),
                                style: Style {
                                    height: Val::Px(40.0),
                                    width: Val::Px(40.0),
                                    ..default()
                                },
                                ..default()
                            },
                        ));

                        grandparent.spawn((
                            VisibleOnParentHover::default(),
                            TextBundle {
                                visibility: Visibility::Hidden,
                                text: Text {
                                    sections: vec![TextSection::new(
                                        item.name.to_string(),
                                        TextStyle {
                                            font_size: 22.0,
                                            color: Color::WHITE,
                                            ..default()
                                        },
                                    )],
                                    ..default()
                                },
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    top: Val::Percent(100.0),
                                    right: Val::Percent(0.0),
                                    ..default()
                                },
                                background_color: Color::BLACK.into(),
                                z_index: ZIndex::Global(10),
                                ..default()
                            },
                        ));

                        if item.amt > 1 {
                            grandparent.spawn(TextBundle {
                                text: Text {
                                    sections: vec![TextSection::new(
                                        item.amt.to_string(),
                                        TextStyle {
                                            font_size: 22.0,
                                            color: Color::WHITE,
                                            ..default()
                                        },
                                    )],
                                    ..default()
                                },
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    bottom: Val::Px(2.0),
                                    right: Val::Px(2.0),
                                    ..default()
                                },
                                background_color: Color::BLACK.into(),
                                ..default()
                            });
                        }
                    });
                }
            }
        });
}

fn spawn_settings_menu_content(
    child_builder: &mut ChildBuilder,
    game_settings: &Res<State<GameSettings>>,
) {
    child_builder.spawn(TextBundle {
        text: Text {
            sections: vec![TextSection::new(
                "Settings",
                TextStyle {
                    font_size: 20.0,
                    ..default()
                },
            )],
            ..default()
        },
        ..default()
    });

    child_builder.spawn(TextBundle {
        text: Text {
            sections: vec![TextSection::new(
                "Render Distance:",
                TextStyle {
                    font_size: 16.0,
                    ..default()
                },
            )],
            ..default()
        },
        style: Style {
            margin: UiRect {
                top: Val::Px(10.0),
                bottom: Val::Px(4.0),
                ..default()
            },
            ..default()
        },
        ..default()
    });

    child_builder
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                justify_content: JustifyContent::SpaceEvenly,
                width: Val::Percent(90.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for i in 0..=5u32 {
                let background_color = if i == game_settings.get().chunk_render_dist.0 {
                    Color::linear_rgba(0.0, 0.0, 0.4, 1.0).into()
                } else {
                    Color::WHITE.into()
                };

                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                display: Display::Flex,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                height: Val::Px(20.0),
                                width: Val::Px(20.0),
                                ..default()
                            },
                            background_color,
                            ..default()
                        },
                        RenderDistButton(i),
                    ))
                    .with_children(|grandparent| {
                        grandparent.spawn(TextBundle {
                            text: Text {
                                sections: vec![TextSection::new(
                                    format!("{}", i),
                                    TextStyle {
                                        font_size: 20.0,
                                        color: Color::BLACK.into(),
                                        ..default()
                                    },
                                )],
                                ..default()
                            },
                            ..default()
                        });
                    });
            }
        });
}

fn change_menu_tabs_background_color(
    mut event_reader: EventReader<StateTransitionEvent<ActiveMenuTab>>,
    mut menu_tab_query: Query<(&MenuTab, &mut BackgroundColor)>,
    active_menu_tab: Res<State<ActiveMenuTab>>,
) {
    for _ in event_reader.read() {
        for (tab, mut background_color) in menu_tab_query.iter_mut() {
            *background_color = get_tab_background_color(tab, active_menu_tab.get());
        }
    }
}

fn get_tab_background_color(tab: &MenuTab, active_menu_tab: &ActiveMenuTab) -> BackgroundColor {
    let red = if *tab == active_menu_tab.0 { 0.3 } else { 0.7 };
    Color::linear_rgba(red, 0.0, 0.0, 1.0).into()
}

fn change_render_dist(
    mut rd_event_writer: EventWriter<RenderDistChanged>,
    button_query: Query<(&RenderDistButton, &Interaction)>,
    game_settings: Res<State<GameSettings>>,
    mut next_game_settings: ResMut<NextState<GameSettings>>,
) {
    let rd = game_settings.get().chunk_render_dist;

    for (button, interaction) in button_query.iter() {
        let rd_does_match = button.0 == rd.0 && button.0 == rd.1 && button.0 == rd.2;
        if *interaction != Interaction::Pressed || rd_does_match {
            continue;
        }

        let mut new_game_settings = game_settings.clone();
        new_game_settings.chunk_render_dist = ChunkRenderDist(button.0, button.0, button.0);

        next_game_settings.set(new_game_settings);
        rd_event_writer.send(RenderDistChanged);

        break;
    }
}

fn change_render_dist_buttons_background_color(
    mut event_reader: EventReader<StateTransitionEvent<GameSettings>>,
    mut buttons_query: Query<(&RenderDistButton, &mut BackgroundColor)>,
    game_settings: Res<State<GameSettings>>,
) {
    for _ in event_reader.read() {
        for (render_dist_button, mut background_color) in buttons_query.iter_mut() {
            if render_dist_button.0 == game_settings.get().chunk_render_dist.0 {
                *background_color = Color::linear_rgba(0.0, 0.0, 0.4, 1.0).into();
            } else {
                *background_color = Color::WHITE.into();
            }
        }
    }
}

fn update_visible_on_parent_hover(
    mut visibility_query: Query<(Entity, &mut Visibility, &VisibleOnParentHover)>,
    interaction_query: Query<&Interaction>,
    parent_query: Query<&Parent>,
) {
    for (entity, mut visibility, voh) in visibility_query.iter_mut() {
        if let Ok(interaction) = interaction_query.get(get_n_parent(entity, &parent_query, 1)) {
            if *interaction == Interaction::Hovered && *visibility != voh.hovered {
                *visibility = voh.hovered;
            } else if *interaction == Interaction::None && *visibility != voh.not_hovered {
                *visibility = voh.not_hovered;
            }
        }
    }
}

fn start_drag_inventory_item(
    inventory_slot_query: Query<(&InventorySlot, &Interaction)>,
    drag_state: Res<State<DragState>>,
    mut next_drag_state: ResMut<NextState<DragState>>,
) {
    for (slot, interaction) in inventory_slot_query.iter() {
        if *interaction == Interaction::Pressed && drag_state.get().0 == Dragging::None {
            next_drag_state.set(DragState(Dragging::InventorySlot(slot.0)));
            break;
        }
    }
}

fn start_drag_equipment_item(
    equipment_slot_query: Query<(&EquipmentSlot, &Interaction)>,
    drag_state: Res<State<DragState>>,
    mut next_drag_state: ResMut<NextState<DragState>>,
) {
    for (slot, interaction) in equipment_slot_query.iter() {
        if *interaction == Interaction::Pressed && drag_state.get().0 == Dragging::None {
            next_drag_state.set(DragState(Dragging::EquipmentSlot(slot.0.clone())));
            break;
        }
    }
}

fn stop_drag_item(
    mut event_writer: EventWriter<InventoryChanged>,
    inventory_slot_query: Query<(&InventorySlot, &RelativeCursorPosition)>,
    equipment_slot_query: Query<(&EquipmentSlot, &RelativeCursorPosition)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut inventory: ResMut<Inventory>,
    drag_state: Res<State<DragState>>,
    mut next_drag_state: ResMut<NextState<DragState>>,
) {
    if mouse.just_released(MouseButton::Left) {
        let mut inventory_changed = false;

        match drag_state.get().0 {
            Dragging::None => {}
            Dragging::InventorySlot(i) => {
                // Swap inventory slots
                for (inventory_slot, rel_cursor_position) in inventory_slot_query.iter() {
                    if rel_cursor_position.mouse_over() {
                        inventory.merge_swap_at(i, inventory_slot.0);
                        inventory_changed = true;
                        break;
                    }
                }

                // Move from inventory slot to equipment slot
                if !inventory_changed {
                    if let Some(item_a) = inventory.slots[i].as_ref() {
                        for (equipment_slot, rel_cursor_position) in equipment_slot_query.iter() {
                            if rel_cursor_position.mouse_over()
                                && item_a.is_equipable_at(&equipment_slot.0)
                            {
                                inventory.equip_at(i, &equipment_slot.0);
                                inventory_changed = true;
                                break;
                            }
                        }
                    }
                }
            }
            Dragging::EquipmentSlot(name) => {
                // Swap equipment slots
                for (equipment_slot, rel_cursor_position) in equipment_slot_query.iter() {
                    if rel_cursor_position.mouse_over() {
                        inventory.equipment.swap(&equipment_slot.0, &name);
                        inventory_changed = true;
                        break;
                    }
                }

                // Move from equipment slot to inventory slot
                if !inventory_changed {
                    for (inventory_slot, rel_cursor_position) in inventory_slot_query.iter() {
                        if rel_cursor_position.mouse_over()
                            && inventory.is_equipable_at(inventory_slot.0, &name)
                        {
                            inventory.equip_at(inventory_slot.0, &name);
                            inventory_changed = true;
                            break;
                        }
                    }
                }
            }
        }

        if inventory_changed {
            event_writer.send(InventoryChanged);
        }
        next_drag_state.set(DragState(Dragging::None));
    }
}

fn use_inventory_item(
    mut item_event_writer: EventWriter<ItemUsed>,
    mut inv_event_writer: EventWriter<InventoryChanged>,
    inventory_slot_query: Query<(&InventorySlot, &RelativeCursorPosition)>,
    player_query: Query<Entity, With<Player>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut inventory: ResMut<Inventory>,
) {
    if mouse.just_released(MouseButton::Right) {
        for (inventory_slot, rel_cursor_position) in inventory_slot_query.iter() {
            if rel_cursor_position.mouse_over() {
                let (output, was_mutated) = inventory.use_at(inventory_slot.0);
                if let Some(item) = output {
                    let entity = player_query.get_single().unwrap();
                    item_event_writer.send(ItemUsed(item, entity));
                }
                if was_mutated {
                    inv_event_writer.send(InventoryChanged);
                }
                break;
            }
        }
    }
}

fn handle_item_used(
    mut event_reader: EventReader<ItemUsed>,
    mut heal_health_event_writer: EventWriter<HealHealth>,
    mut heal_stamina_event_writer: EventWriter<HealStamina>,
    mut take_dmg_event_writer: EventWriter<TakeDamage>,
    mut health_query: Query<(Entity, &mut Health)>,
    mut stamina_query: Query<(Entity, &mut Stamina)>,
) {
    for event in event_reader.read() {
        match event.0.name {
            ItemName::HealthPotion => match health_query.iter_mut().find(|(e, _)| *e == event.1) {
                Some((e, _)) => {
                    heal_health_event_writer.send(HealHealth(30.0, e));
                }
                None => {
                    should_not_happen!("using health potion on entity w/o health component");
                }
            },
            ItemName::StaminaPotion => match health_query.iter_mut().find(|(e, _)| *e == event.1) {
                Some((e, _)) => {
                    heal_stamina_event_writer.send(HealStamina(30.0, e));
                }
                None => {
                    should_not_happen!("using stamina potion on entity w/o stamina component");
                }
            },
            ItemName::HealthRegenPotion => match health_query
                .iter_mut()
                .find(|(e, _)| *e == event.1)
            {
                Some((_, mut health)) => {
                    health.add_temp_modifier(0.05, 90);
                }
                None => {
                    should_not_happen!("using health regen potion on entity w/o health component");
                }
            },
            ItemName::StaminaRegenPotion => {
                match stamina_query.iter_mut().find(|(e, _)| *e == event.1) {
                    Some((_, mut stamina)) => {
                        stamina.add_temp_modifier(0.05, 90);
                    }
                    None => {
                        should_not_happen!(
                            "using stamina regen potion on entity w/o stamina component"
                        );
                    }
                }
            }
            ItemName::HealthPoison => match health_query.iter_mut().find(|(e, _)| *e == event.1) {
                Some((e, _)) => {
                    take_dmg_event_writer.send(TakeDamage(vec![(DmgType::Poison, 30.0)], e));
                }
                None => {
                    should_not_happen!("using health poison on entity w/o health component");
                }
            },
            ItemName::StaminaPoison => {
                match stamina_query.iter_mut().find(|(e, _)| *e == event.1) {
                    Some((e, _)) => {
                        take_dmg_event_writer.send(TakeDamage(vec![(DmgType::Stamina, 30.0)], e));
                    }
                    None => {
                        should_not_happen!("using stamina poison on entity w/o stamina component");
                    }
                }
            }
            ItemName::HealthRegenPoison => match health_query
                .iter_mut()
                .find(|(e, _)| *e == event.1)
            {
                Some((_, mut health)) => {
                    health.add_temp_modifier(-0.05, 90);
                }
                None => {
                    should_not_happen!("using health regen poison on entity w/o health component");
                }
            },
            ItemName::StaminaRegenPoison => {
                match stamina_query.iter_mut().find(|(e, _)| *e == event.1) {
                    Some((_, mut stamina)) => {
                        stamina.add_temp_modifier(-0.05, 90);
                    }
                    None => {
                        should_not_happen!(
                            "using stamina regen poison on entity w/o stamina component"
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn update_item_image_cursor_follower(
    mut commands: Commands,
    mut event_reader: EventReader<StateTransitionEvent<DragState>>,
    cursor_follower_query: Query<Entity, (With<ItemImageCursorFollower>, With<CursorFollower>)>,
    asset_server: Res<AssetServer>,
    cursor_position: Res<CursorPosition>,
    inventory: Res<Inventory>,
    drag_state: Res<State<DragState>>,
) {
    for _ in event_reader.read() {
        match drag_state.get().0 {
            Dragging::InventorySlot(i) => {
                if let Some(Some(item)) = inventory.slots.get(i) {
                    spawn_item_image_cursor_follower(
                        &mut commands,
                        &cursor_position,
                        &item.ui_image(&asset_server),
                        &item_style(),
                    );
                }
            }
            Dragging::EquipmentSlot(name) => {
                if let Some(item) = inventory.equipment.at(&name) {
                    spawn_item_image_cursor_follower(
                        &mut commands,
                        &cursor_position,
                        &item.ui_image(&asset_server),
                        &item_style(),
                    );
                }
            }
            Dragging::None => {
                for entity in cursor_follower_query.iter() {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }

        break;
    }
}

fn spawn_item_image_cursor_follower(
    commands: &mut Commands,
    cursor_position: &CursorPosition,
    ui_image: &UiImage,
    ui_image_style: &Style,
) {
    let mut image = ui_image.clone();
    image.color = Color::srgba_u8(255, 255, 255, 200);

    let mut style = ui_image_style.clone();
    style.left = Val::Px(cursor_position.0.x);
    style.top = Val::Px(cursor_position.0.y);

    commands.spawn((
        ItemImageCursorFollower,
        CursorFollower,
        ImageBundle {
            image,
            style,
            ..default()
        },
    ));
}

fn item_style() -> Style {
    Style {
        height: Val::Px(40.0),
        width: Val::Px(40.0),
        ..default()
    }
}
