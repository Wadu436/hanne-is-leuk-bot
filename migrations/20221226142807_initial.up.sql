-- Add up migration script here
CREATE TABLE guilds (
    guild_id INT8 PRIMARY KEY NOT NULL,
    message_channel_id INT8 NOT NULL,
    message_time TIME NOT NULL,
    message_timezone TEXT NOT NULL,
    format TEXT NOT NULL,
    CONSTRAINT time_check check ((message_time is null) = (message_timezone is null))
);

CREATE TABLE exams (
    exam_id BIGSERIAL PRIMARY KEY,
    user_id INT8 NOT NULL,
    guild_id INT8 NOT NULL,
    day DATE NOT NULL,
    exam_name TEXT NOT NULL,
    UNIQUE(user_id, day, exam_name, guild_id),
    FOREIGN KEY (guild_id)
        REFERENCES guilds (guild_id)
            ON DELETE CASCADE
);