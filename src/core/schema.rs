// @generated automatically by Diesel CLI.

diesel::table! {
    captures (id) {
        id -> Int4,
        uuid -> Uuid,
        url -> Text,
        time_initiated -> Timestamptz,
        owner -> Int4,
        public -> Bool,
    }
}

diesel::table! {
    extracts (id) {
        id -> Int4,
        uuid -> Uuid,
        capture -> Int4,
        extractor -> Text,
        success -> Bool,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Text,
        passhash -> Text,
    }
}

diesel::joinable!(captures -> users (owner));
diesel::joinable!(extracts -> captures (capture));

diesel::allow_tables_to_appear_in_same_query!(captures, extracts, users,);
