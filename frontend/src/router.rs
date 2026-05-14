use yew::prelude::*;
use yew_router::prelude::*;

use crate::pages::console::ConsolePage;
use crate::pages::database::DatabasePage;
use crate::pages::home::HomePage;
use crate::pages::not_found::NotFoundPage;
use crate::pages::table_browse::TableBrowsePage;
use crate::pages::table_structure::TableStructurePage;

#[derive(Clone, Routable, PartialEq, Debug)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/console")]
    Console,
    #[at("/db/:db")]
    Database { db: String },
    #[at("/db/:db/table/:table/structure")]
    Structure { db: String, table: String },
    #[at("/db/:db/table/:table/browse")]
    Browse { db: String, table: String },
    #[not_found]
    #[at("/404")]
    NotFound,
}

pub fn switch(route: Route) -> Html {
    match route {
        Route::Home => html! { <HomePage /> },
        Route::Console => html! { <ConsolePage /> },
        Route::Database { db } => html! { <DatabasePage {db} /> },
        Route::Structure { db, table } => html! { <TableStructurePage {db} {table} /> },
        Route::Browse { db, table } => html! { <TableBrowsePage {db} {table} /> },
        Route::NotFound => html! { <NotFoundPage /> },
    }
}
