CREATE TABLE presents (
    id BIGSERIAL NOT NULL,
    game_id uuid NOT NULL,
    name TEXT NOT NULL,
    wrapped_images TEXT [] NOT NULL DEFAULT '{}' check (array_position(wrapped_images, null) is null),
    unwrapped_images TEXT [] NOT NULL DEFAULT '{}' check (array_position(unwrapped_images, null) is null),
    player_id BIGINT,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp,
    PRIMARY KEY (id),
    CONSTRAINT fk_player FOREIGN KEY (player_id) REFERENCES players(id),
    CONSTRAINT fk_game FOREIGN KEY (game_id) REFERENCES games(id)
);
