//! Pad Frontend Entrypoint
//!
//! This module declares the component submodules of the Yew client
//! and initializes the web application renderer with the main `App` component.
//! It serves as the application crate root.
//!
//! Copyright (c) 2026 Pad Authors. All rights reserved.
//!
//! Licensed under the Apache License, Version 2.0 (the "License");
//! you may not use this file except in compliance with the License.
//! You may obtain a copy of the License at
//!
//!     http://www.apache.org/licenses/LICENSE-2.0

mod app;
mod collab;
mod collab_utils;
mod components;
mod api;
mod i18n;
mod storage;
mod types;

use app::App;

fn main() {
    // Render the App component into the root DOM element
    yew::Renderer::<App>::new().render();
}
