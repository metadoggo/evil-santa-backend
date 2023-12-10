DROP TRIGGER tr_notify_play_event ON play_events;
DROP FUNCTION notify_play_event;
DROP TABLE play_events CASCADE;

ALTER TABLE games DROP CONSTRAINT fk_present;
ALTER TABLE games DROP CONSTRAINT fk_player;
ALTER TABLE games DROP column present_id;
ALTER TABLE games DROP column player_id;
ALTER TABLE games DROP column started_at;
