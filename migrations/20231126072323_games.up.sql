--
-- Tables
--
CREATE TABLE games (
    id uuid NOT NULL DEFAULT gen_random_uuid (),
    name TEXT NOT NULL,
    images TEXT [] NOT NULL DEFAULT '{}' check (array_position(images, null) is null),
    users JSONB NOT NULL,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp,
    PRIMARY KEY (id)
);
