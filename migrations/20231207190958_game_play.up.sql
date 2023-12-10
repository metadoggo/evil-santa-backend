ALTER TABLE games ADD column started_at timestamp;
ALTER TABLE games ADD column player_id BIGINT;
ALTER TABLE games ADD column present_id BIGINT;
ALTER TABLE games ADD CONSTRAINT fk_player FOREIGN KEY (player_id) REFERENCES players(id);
ALTER TABLE games ADD CONSTRAINT fk_present FOREIGN KEY (present_id) REFERENCES presents(id);

--
-- Tables
--

CREATE TABLE play_events (
    id BIGSERIAL NOT NULL,
    game_id uuid NOT NULL,
    player_id BIGINT NOT NULL,
    present_id BIGINT,
    from_player_id BIGINT,
    from_present_id BIGINT,
    created_at timestamp NOT NULL DEFAULT now(),
    PRIMARY KEY (id),
    CONSTRAINT fk_player FOREIGN KEY (player_id) REFERENCES players(id),
    CONSTRAINT fk_present FOREIGN KEY (present_id) REFERENCES presents(id),
    CONSTRAINT fk_from_player FOREIGN KEY (from_player_id) REFERENCES players(id),
    CONSTRAINT fk_from_present FOREIGN KEY (from_present_id) REFERENCES presents(id),
    CONSTRAINT fk_game FOREIGN KEY (game_id) REFERENCES games(id)
);

--
-- Notify external listeners on play event created
--
CREATE FUNCTION notify_play_event()
RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('play', row_to_json(NEW) :: text);
    RETURN NEW;
END;

$$ LANGUAGE PLPGSQL;

--

CREATE TRIGGER tr_notify_play_event
AFTER INSERT
ON play_events
FOR EACH ROW
    EXECUTE PROCEDURE notify_play_event();
