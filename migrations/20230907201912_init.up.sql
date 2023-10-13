-- Add up migration script here

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
--DROP TABLE IF EXISTS accounts;
DROP TYPE IF EXISTS user_type;
DROP TYPE IF EXISTS consultant_specialty;
DROP TYPE IF EXISTS consultant_territory;
DROP TYPE IF EXISTS state_abbr;
DROP TYPE IF EXISTS state_name;
DROP TYPE IF EXISTS us_territories;
DROP TYPE IF EXISTS attachment_channel;
DROP TYPE IF EXISTS mime_type;

-- CREATE TYPE consultant_specialty AS ENUM ('Insurance', 'Finance', 'Government');

-- CREATE TYPE mime_type AS ENUM ('application/pdf', 'application/json', 'video/mp4', 'image/jpeg', 'image/svg+xml', 'image/gif', 'image/png');

-- CREATE TYPE attachment_channel AS ENUM ('Email', 'Upload');

-- CREATE TYPE consultant_territory AS ENUM ('Midwest', 'East', 'West', 'North', 'South');

-- CREATE TYPE state_abbr AS ENUM ('AL','AK','AZ','AR','CA','CO','CT','DE','FL','GA','HI','ID','IL','IN','IA','KS','KY','LA','ME','MD','MA',
        -- 'MI','MN','MS','MO','MT','NE','NV','NH','NJ','NM','NY','NC','ND','OH','OK','OR','PA','RI','SC','SD','TN',
        -- 'TX','UT','VT','VA','WA','WV','WI','WY','AS','GU','MP','PR','VI','DC');

-- CREATE TYPE state_name AS ENUM ('Alabama','Alaska','Arizona','Arkansas','California','Colorado','Connecticut','Delaware','Florida','Georgia',
        -- 'Hawaii','Idaho','Illinois','Indiana','Iowa','Kansas','Kentucky','Louisiana','Maine','Maryland','Massachusetts',
        -- 'Michigan','Minnesota','Mississippi','Missouri','Montana','Nebraska','Nevada','New_Hampshire','New_Jersey','New_Mexico',
        -- 'New_York','North_Carolina','North_Dakota','Ohio','Oklahoma','Oregon','Pennsylvania','Rhode_Island','South_Carolina',
        -- 'South_Dakota','Tennessee','Texas','Utah','Vermont','Virginia','Washington','West_Virginia','Wisconsin','Wyoming');

-- CREATE TYPE us_territories AS ENUM ('American_Samoa', 'Guam', 'Northern_Mariana_Islands', 'Puerto_Rico', 'Virgin_Islands', 'District_of_Columbia');

CREATE TYPE user_type AS ENUM (
       'guest',
       'regular',
       'admin'
);

CREATE TABLE IF NOT EXISTS accounts (
        account_id SERIAL PRIMARY KEY,
        account_name TEXT NOT NULL UNIQUE,
        account_secret TEXT DEFAULT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW()
    );

CREATE TABLE IF NOT EXISTS users (
        user_id SERIAL PRIMARY KEY,
        account_id INTEGER NOT NULL DEFAULT 3,
        username TEXT NOT NULL UNIQUE,
        email TEXT NOT NULL UNIQUE,
        secret TEXT DEFAULT NULL,
        password TEXT NOT NULL,
        avatar_path TEXT NOT NULL DEFAULT '/images/default_avatar.svg',
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_account_id
            FOREIGN KEY(account_id) 
	            REFERENCES accounts(account_id)
    );

CREATE TABLE IF NOT EXISTS user_settings (
        user_settings_id SERIAL PRIMARY KEY,
        user_id INTEGER NOT NULL,
        theme_id INTEGER NOT NULL DEFAULT 1,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        CONSTRAINT fk_user_id
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id)
    );

CREATE TABLE IF NOT EXISTS user_sessions (
        user_session_id SERIAL PRIMARY KEY,
        user_id INTEGER NOT NULL,
        session_id TEXT NOT NULL,
        -- session_id TEXT NOT NULL DEFAULT (uuid_generate_v4()),
        expires TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        logout BOOLEAN NOT NULL DEFAULT FALSE,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NULL,
        CONSTRAINT fk_user_id
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id)
    );

CREATE TABLE IF NOT EXISTS specialties (
        specialty_id SERIAL PRIMARY KEY,
        specialty_name TEXT NOT NULL
    );

CREATE TABLE IF NOT EXISTS territories (
        territory_id SERIAL PRIMARY KEY,
        territory_name TEXT NOT NULL,
        territory_states TEXT[] NULL
    );

CREATE TABLE IF NOT EXISTS consultants (
        consultant_id SERIAL PRIMARY KEY,
        consultant_slug TEXT NOT NULL DEFAULT (uuid_generate_v4()),
        -- specialty consultant_specialty NOT NULL,
        -- territory consultant_territory NULL,
        consultant_f_name TEXT NOT NULL,
        consultant_l_name TEXT NOT NULL,
        specialty_id INTEGER NOT NULL,
        territory_id INTEGER NULL,
        user_id INTEGER NOT NULL,
        img_path TEXT DEFAULT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_user
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id),
        CONSTRAINT fk_specialty
            FOREIGN KEY(specialty_id) 
	            REFERENCES specialties(specialty_id),
        CONSTRAINT fk_territory
            FOREIGN KEY(territory_id) 
	            REFERENCES territories(territory_id)
    );

CREATE TABLE IF NOT EXISTS consultant_ties (
        consultant_tie_id SERIAL PRIMARY KEY,
        consultant_id INTEGER NOT NULL,
        specialty_id INTEGER NOT NULL,
        territory_id INTEGER NULL,
        consultant_start DATE NOT NULL DEFAULT CURRENT_DATE,
        consultant_end DATE DEFAULT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_consultant
            FOREIGN KEY(consultant_id) 
	            REFERENCES consultants(consultant_id),
        CONSTRAINT fk_specialty
            FOREIGN KEY(specialty_id) 
	            REFERENCES specialties(specialty_id),
        CONSTRAINT fk_territory
            FOREIGN KEY(territory_id) 
	            REFERENCES territories(territory_id)
    );

CREATE TABLE IF NOT EXISTS clients (
        client_id SERIAL PRIMARY KEY,
        client_slug TEXT NOT NULL DEFAULT (uuid_generate_v4()),
        client_f_name TEXT NULL,
        client_l_name TEXT NULL,

        client_company_name TEXT DEFAULT NULL,

        client_address_one TEXT NOT NULL,
        client_address_two TEXT NULL,
        client_city TEXT NOT NULL,
        -- client_state state_abbr NOT NULL,
        client_state CHAR(2) NOT NULL,
        client_zip VARCHAR (5) NOT NULL,
        client_dob DATE NULL, 
        client_primary_phone TEXT NULL,
        client_mobile_phone TEXT NULL,
        client_secondary_phone TEXT NULL,
        client_email TEXT NULL,
        account_id INTEGER NOT NULL,
        user_id INTEGER NOT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_user
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id),
        CONSTRAINT fk_account
            FOREIGN KEY(account_id) 
	            REFERENCES accounts(account_id)
    );

CREATE TABLE IF NOT EXISTS contacts (
        contact_id SERIAL PRIMARY KEY,
        contact_title TEXT NULL,
        contact_f_name TEXT NOT NULL,
        contact_l_name TEXT NULL,
        contact_email TEXT NOT NULL,
        contact_primary_phone TEXT NULL,
        contact_secondary_phone TEXT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW()
    );


-- FIXME: Add PostGIS and lat/long
CREATE TABLE IF NOT EXISTS locations (
        location_id SERIAL PRIMARY KEY,
        location_slug TEXT NOT NULL DEFAULT (uuid_generate_v4()),
        location_name TEXT NOT NULL,
        location_address_one TEXT NOT NULL,
        location_address_two TEXT NULL,
        location_city TEXT NOT NULL,
        location_state CHAR(2) NOT NULL,
        location_zip VARCHAR (5) NOT NULL,
        location_phone TEXT NULL,
        location_contact_id INTEGER DEFAULT 1,
        territory_id INTEGER NOT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_contact
            FOREIGN KEY(location_contact_id) 
	            REFERENCES contacts(contact_id),
        CONSTRAINT fk_territory
            FOREIGN KEY(territory_id) 
	            REFERENCES territories(territory_id)
    );

CREATE TABLE IF NOT EXISTS engagements (
        engagement_id SERIAL PRIMARY KEY,
        rating INTEGER NOT NULL,
        text TEXT NOT NULL UNIQUE,
        user_id INTEGER DEFAULT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_user
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id)
    );

CREATE TABLE IF NOT EXISTS messages (
        message_id SERIAL PRIMARY KEY,
        content TEXT NOT NULL,
        subject TEXT NOT NULL,
        sent_to INTEGER NOT NULL,
        sent_from INTEGER NOT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        sent_at TIMESTAMPTZ DEFAULT NULL,
        read_at TIMESTAMPTZ DEFAULT NULL,
        CONSTRAINT fk_sent_to
            FOREIGN KEY(sent_to) 
	            REFERENCES users(user_id),
        CONSTRAINT fk_sent_from
            FOREIGN KEY(sent_from) 
	            REFERENCES users(user_id)
    );

CREATE TABLE IF NOT EXISTS attachments (
        attachment_id SERIAL PRIMARY KEY,
        path TEXT UNIQUE NOT NULL,
        user_id INTEGER NOT NULL,
        -- mime_type mime_type NOT NULL,
        -- channel attachment_channel NOT NULL,
        mime_type TEXT NOT NULL,
        channel TEXT NOT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_user_id
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id)
    );

CREATE TABLE IF NOT EXISTS consults (
        consult_id SERIAL PRIMARY KEY,
        consultant_id INTEGER NOT NULL,
        client_id INTEGER NOT NULL,
        location_id INTEGER NOT NULL,
        consult_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        consult_end TIMESTAMPTZ DEFAULT NULL,
        notes TEXT DEFAULT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NULL,
        CONSTRAINT fk_client
            FOREIGN KEY(client_id) 
	            REFERENCES clients(client_id),
        CONSTRAINT fk_location
            FOREIGN KEY(location_id) 
	            REFERENCES locations(location_id),
        CONSTRAINT fk_consultant
            FOREIGN KEY(consultant_id) 
	            REFERENCES consultants(consultant_id)
    );

INSERT INTO territories (territory_id, territory_name, territory_states)
VALUES
(1, 'Northeast', ARRAY['NY', 'MA','VT', 'NH', 'RI']),
(2, 'Southeast', ARRAY['FL', 'GA','LA', 'VA', 'WV']),
(3, 'West', ARRAY['CA', 'WA','OR', 'NV', 'NM', 'AZ', 'WY', 'ID']),
(4, 'Midwest', ARRAY['NE', 'KS','OK', 'TX', 'IA', 'CO']);

INSERT INTO specialties (specialty_id, specialty_name)
VALUES
(1, 'Finance'),
(2, 'Insurance'),
(3, 'Technology'),
(4, 'Government');

INSERT INTO accounts (account_name, account_secret) 
VALUES 
('root', 'root_secret'),
('admin', 'admin_secret'),
('default_user', 'user_secret'),
('default_client', 'client_secret'),
('default_company_client', 'company_client_secret');

INSERT INTO users (username, account_id, email, password) 
VALUES 
('root', 1, 'root@consultancy.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('admin', 2, 'admin@consultancy.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Users
('jim_jam', 2, 'jim@jam.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('aaron', 2, 'aaron@aaron.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Clients
('first_client', 3, 'client_one@client.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('second_client', 3, 'client_two@client.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Consultants
('first_consultant', 2, 'consultant_one@consultancy.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('second_consultant', 2, 'consultant_two@consultancy.com', '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8');

INSERT INTO user_settings (user_id, theme_id) 
VALUES 
(1, 1),
(2, 1),
(3, 1),
(4, 1),
(5, 1),
(6, 1),
(7, 1),
(8, 1);

INSERT INTO user_sessions (user_id, session_id, expires, created_at) 
VALUES 
(1, 'c4689973-82eb-404a-a249-e684cadb31df', NOW() - '20 days'::interval, NOW() - '21 days'::interval),
(1, '7d9527cb-44e5-4f2d-813f-6d2ed5ed92a2', NOW() - '15 days'::interval, NOW() - '16 days'::interval),
(2, 'cb8984a0-d6cb-4f4c-8dc2-0209c5b5f027', NOW() - '14 days'::interval, NOW() - '15 days'::interval);

INSERT INTO clients (client_f_name, client_l_name, client_company_name, client_primary_phone, client_address_one, client_city, client_state, client_zip, client_dob, account_id, user_id) 
VALUES 
('Mike', 'Ryan', NULL, '555-555-5555', '1111 Client St.', 'Client City', 'NE', '68114', '1989-01-08', 3, 5),
(NULL, NULL, 'McGillicuddy & Sons LLC', '555-555-5555', '1111 Jupiter St.', 'Company Town', 'NE', '68114', NULL, 4, 5),
('Chris', 'Cote', NULL, '555-555-5555', '2222 Client St.', 'Client Town', 'MN', '55057', '1966-07-22', 3, 6);

INSERT INTO consultants (consultant_f_name, consultant_l_name, specialty_id, user_id, img_path) 
VALUES 
('Terry', 'Bolea', 1, 7, '/img/consultants/consultant_one.svg'),
('Joe', 'Zagacki', 2, 8, '/img/consultants/consultant_two.svg');

INSERT INTO consultant_ties (consultant_id, specialty_id, territory_id, consultant_start, consultant_end) 
VALUES 
(1, 2, NULL, '2022-02-02', '2023-02-02'),
(2, 1, NULL, '2022-02-02', '2023-02-02'),
(1, 1, NULL, '2023-02-02', NULL),
(2, 2, NULL, '2023-02-02', NULL);

INSERT INTO contacts (contact_title, contact_f_name, contact_l_name, contact_email, contact_primary_phone, contact_secondary_phone) 
VALUES 
('Site Admin', 'Greg', 'Cote', 'cote@gregslobos.com', '555-555-5555', '555-555-5555'),
('Location Manager', 'Billy', 'Gil', 'bill@marlins.com', '555-555-5555', '555-555-5555');

INSERT INTO locations (location_name, location_address_one, location_address_two, location_city, location_state, location_zip, location_phone, location_contact_id, territory_id) 
VALUES 
('Default - Main Office', '1234 Main St.', NULL, 'Omaha', 'NE', '68114', '555-555-5555', DEFAULT, 4),
('Bend Conference Center', '5432 Postgres Ave', 'Ste. 101', 'Bend', 'OR', '97701', '555-555-5555', DEFAULT, 3),
('Austin Heights', '6379 Redis Lane', NULL, 'Austin', 'TX', '78799', '555-555-5555', 2, 4);

INSERT INTO engagements (rating, text, user_id) 
VALUES 
(7, 'It was a seven.', 1),
(3, 'I give it a 3', 2);

INSERT INTO consults (consultant_id, client_id, location_id, consult_start, consult_end, notes) 
VALUES 
(1, 1, 2, '2023-09-11 19:10:25-06', '2023-09-11 19:30:25-06', NULL),
(2, 2, 1, '2023-09-11 16:00:25-06', '2023-09-11 16:50:11-06', 'Using the Default Address. Location not persisted. Location was at the Clevelander.');

INSERT INTO attachments (path, mime_type, user_id, channel, created_at, updated_at) 
VALUES 
('https://upload.wikimedia.org/wikipedia/commons/5/5d/Kuchnia_polska-p243b.png', 'image/png', 3, 'Upload', '2023-09-11 19:10:25-06', '2023-09-11 19:30:25-06'),
('https://upload.wikimedia.org/wikipedia/commons/f/f5/Kuchnia_polska-p35b.png', 'image/png', 4, 'Email', '2023-09-11 16:00:25-06', '2023-09-11 16:50:11-06');