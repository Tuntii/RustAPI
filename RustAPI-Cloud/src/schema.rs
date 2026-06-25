diesel::table! {
    users (id) {
        id -> Varchar,
        github_id -> Int8,
        login -> Varchar,
        avatar_url -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        tier -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    oauth_devices (id) {
        id -> Varchar,
        device_code -> Varchar,
        user_code -> Varchar,
        user_id -> Nullable<Varchar>,
        client_id -> Varchar,
        scopes -> Varchar,
        expires_at -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    projects (id) {
        id -> Varchar,
        user_id -> Varchar,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    deploys (id) {
        id -> Varchar,
        project_id -> Varchar,
        user_id -> Varchar,
        binary_path -> Varchar,
        status -> Varchar,
        url -> Nullable<Varchar>,
        port -> Nullable<Int4>,
        pid -> Nullable<Int4>,
        error_message -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(users, oauth_devices, projects, deploys);
