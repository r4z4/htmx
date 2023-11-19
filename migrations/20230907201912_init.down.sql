-- Add down migration script here
DROP TABLE IF EXISTS consultant_ties;

DROP TABLE IF EXISTS article_categories;
DROP TABLE IF EXISTS engagements;
DROP TABLE IF EXISTS consults;
-- Tables depends on these
DROP TABLE IF EXISTS clients;
DROP TABLE IF EXISTS consultants;

DROP TABLE IF EXISTS reset_password_requests;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS attachments;
DROP TABLE IF EXISTS locations;
DROP TABLE IF EXISTS contacts;
DROP TABLE IF EXISTS territories;
DROP TABLE IF EXISTS specialties;
DROP TABLE IF EXISTS mime_types;
DROP TABLE IF EXISTS entities; 
DROP TABLE IF EXISTS contact_submissions;
DROP TABLE IF EXISTS article_submissions;
DROP TABLE IF EXISTS states; 

DROP TABLE IF EXISTS consutlants;
DROP TABLE IF EXISTS user_sessions;
DROP TABLE IF EXISTS user_types;
DROP TABLE IF EXISTS user_settings;
DROP TABLE IF EXISTS user_details;
DROP TABLE IF EXISTS users;
-- This needs to be last
DROP TABLE IF EXISTS accounts;

DROP TRIGGER IF EXISTS user_settings_insert_trigger ON users;


DROP TYPE IF EXISTS consultant_specialty;

DROP TYPE IF EXISTS consult_purposes;
DROP TYPE IF EXISTS consult_results;
DROP TYPE IF EXISTS client_types;
DROP TYPE IF EXISTS consultant_territory;
DROP TYPE IF EXISTS user_type;
DROP TYPE IF EXISTS state_abbr;
DROP TYPE IF EXISTS state_name;
DROP TYPE IF EXISTS us_territories;
DROP TYPE IF EXISTS attachment_channel;
DROP TYPE IF EXISTS mime_type;