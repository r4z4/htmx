CREATE OR REPLACE FUNCTION user_settings_insert_trigger_fnc()

  RETURNS trigger AS

$$

BEGIN



    INSERT INTO "user_settings" ( "user_settings_id", "user_id")

         VALUES(DEFAULT,NEW."user_id");


RETURN NEW;

END;

$$

LANGUAGE 'plpgsql';



CREATE TRIGGER user_settings_insert_trigger

  AFTER INSERT

  ON "users"

  FOR EACH ROW

  EXECUTE PROCEDURE user_settings_insert_trigger_fnc();