--
-- Tables
--
CREATE TABLE players (
    id BIGSERIAL NOT NULL,
    game_id uuid NOT NULL,
    name TEXT NOT NULL,
    images TEXT [] NOT NULL DEFAULT '{}' check (array_position(images, null) is null),
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp,
    PRIMARY KEY (id),
    CONSTRAINT fk_game FOREIGN KEY (game_id) REFERENCES games(id)
);
