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

-- When I forget to add it to DOWN file
-- DROP TABLE IF EXISTS user_types;

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

-- CREATE TYPE user_type AS ENUM (
--        'guest',
--        'regular',
--        'subadmin',
--        'admin'
-- );

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
        user_type_id INT NOT NULL DEFAULT 2,
        secret TEXT DEFAULT NULL,
        password TEXT NOT NULL,
        avatar_path TEXT NOT NULL DEFAULT '/images/default_avatar.svg',
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_account_id
            FOREIGN KEY(account_id) 
	            REFERENCES accounts(account_id)
    );

CREATE TABLE IF NOT EXISTS user_details (
        user_details_id SERIAL PRIMARY KEY,
        user_id INTEGER NOT NULL,
        address_one TEXT NOT NULL,
        address_two TEXT NULL,
        city TEXT NOT NULL,
        state CHAR(2) NOT NULL,
        zip VARCHAR (5) NOT NULL,
        dob DATE NOT NULL,
        primary_phone TEXT NOT NULL,
        mobile_phone TEXT NULL,
        secondary_phone TEXT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        CONSTRAINT fk_user_id
            FOREIGN KEY(user_id) 
	            REFERENCES users(user_id)
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

CREATE TABLE IF NOT EXISTS entities (
        entity_id SERIAL PRIMARY KEY,
        entity_name TEXT NOT NULL
    );

CREATE TABLE IF NOT EXISTS mime_types (
        mime_type_id SERIAL PRIMARY KEY,
        mime_type_name TEXT NOT NULL
    );

CREATE TABLE IF NOT EXISTS user_types (
        user_type_id SERIAL PRIMARY KEY,
        user_type_name TEXT NOT NULL UNIQUE
    );


CREATE TABLE IF NOT EXISTS territories (
        territory_id SERIAL PRIMARY KEY,
        territory_name TEXT NOT NULL,
        territory_states TEXT[] NULL
    );

CREATE TABLE IF NOT EXISTS consultants (
        consultant_id SERIAL PRIMARY KEY,
        user_id INTEGER NOT NULL,
        consultant_slug TEXT NOT NULL DEFAULT (uuid_generate_v4()),
        -- specialty consultant_specialty NOT NULL,
        -- territory consultant_territory NULL,
        consultant_f_name TEXT NOT NULL,
        consultant_l_name TEXT NOT NULL,
        specialty_id INTEGER NOT NULL,
        territory_id INTEGER NOT NULL DEFAULT 1,
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
        territory_id INTEGER NOT NULL DEFAULT 1,
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
        mime_type_id INTEGER NOT NULL,
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
        consult_attachments INTEGER[] DEFAULT NULL,
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
(1, 'national',     NULL),
(2, 'northeast',    ARRAY['DE', 'MD', 'PA', 'NJ', 'NY', 'MA', 'CT', 'VT', 'NH', 'RI', 'ME', 'OH']),
(3, 'southeast',    ARRAY['AR', 'LA', 'MS', 'TN', 'AL', 'KY', 'WV', 'VA', 'NC', 'SC', 'GA', 'FL']),
(4, 'west',         ARRAY['CA', 'WA', 'OR', 'NV', 'AZ', 'NM', 'UT', 'WY', 'ID', 'MT', 'AK', 'CO', 'WY']),
(5, 'midwest',      ARRAY['NE', 'IA', 'KS', 'OK', 'MO', 'SD', 'ND', 'MN', 'WI', 'MI', 'IN', 'IL', 'TX']);

INSERT INTO specialties (specialty_id, specialty_name)
VALUES
(1, 'finance'),
(2, 'insurance'),
(3, 'technology'),
(4, 'government');

INSERT INTO user_types (user_type_id, user_type_name)
VALUES
(1, 'admin'),
(2, 'subadmin'),
(3, 'regular'),
(4, 'guest');

INSERT INTO entities (entity_id, entity_name)
VALUES
(1, 'user'),
(2, 'admin'),
(3, 'subadmin'),
(4, 'consultant'),
(5, 'location'),
(6, 'consult'),
(7, 'client');

INSERT INTO mime_types (mime_type_id, mime_type_name)
VALUES
(1, 'image/png'),
(2, 'image/jpeg'),
(3, 'image/gif'),
(4, 'image/webp'),
(5, 'image/svg+xml'),
(6, 'audio/wav'),
(7, 'audio/mpeg'),
(8, 'audio/webm'),
(9, 'video/webm'),
(10, 'video/mpeg'),
(11, 'video/mp4'),
(12, 'application/json'),
(13, 'application/pdf'),
(14, 'text/csv'),
(15, 'text/html'),
(16, 'text/calendar');

INSERT INTO accounts (account_name, account_secret) 
VALUES 
('root',                    'root_secret'),
('admin',                   'admin_secret'),
('default_user',            'user_secret'),
('default_client',          'client_secret'),
('default_company_client',  'company_client_secret');

INSERT INTO users (username, user_type_id, account_id, email, password) 
VALUES 
('root',                1,    1, 'root@consultancy.com',           '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('admin',               1,    2, 'admin@consultancy.com',          '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Users
('jim_jam',             3,    2, 'jim@jam.com',                    '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('aaron',               3,    2, 'aaron@aaron.com',                '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Clients
('first_client',        3,  3, 'client_one@client.com',            '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('second_client',       3,  3, 'client_two@client.com',            '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Subadmins
('sudadmin_one',        2, 3, 'subadmin_one@subadmin.com',         '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
-- Consultants
('hulk_hogan',          2, 2, 'hulk_hogan@consultancy.com',        '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('mike_ryan',           2, 2, 'mike_ryan@consultancy.com',         '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('zardos',              2, 2, 'zardos@consultancy.com',            '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('gregs_lobos',         2, 2, 'gregs_lobos@consultancy.com',       '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('rob_bower',           2, 2, 'rob_bower@consultancy.com',         '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('v_smith',             2, 2, 'v_smith@consultancy.com',           '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8'),
('joe_z',               2, 2, 'joe_z@consultancy.com',             '$argon2id$v=19$m=4096,t=192,p=12$l+EgZvJ/+GM1vOg3tNFD6dzeQtfGQiRA1bZLC/MBu/k$wU8nUrHybUQr25Un9CsCDKuWK9R8lLxKCH+Xp/P79l8');

INSERT INTO user_details (user_id, address_one, address_two, city, state, zip, dob, primary_phone) 
VALUES 
(7, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(8, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(9, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(10, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(11, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(12, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(13, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333'),
(14, '12 Subadmin Dr', NULL, 'Omaha', 'NE', '68124', '1980-01-05', '402-333-3333');

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
('Mike',    'Ryan',     NULL,                       '555-555-5555', '1111 Client St.',      'Client City',      'NE', '68114', '1989-01-08',    3, 5),
(NULL,      NULL,       'McGillicuddy & Sons LLC',  '555-555-5555', '1111 Jupiter St.',     'Company Town',     'NE', '68114', NULL,            4, 5),
('Chris',   'Cote',     NULL,                       '555-555-5555', '2222 Client St.',      'Client Town',      'MN', '55057', '1966-07-22',    3, 6),
('Tobias',  'Funke',    NULL,                       '555-555-5555', '123 Haliburton Dr.',   'Los Angeles',      'CA', '90005', '1989-01-08',    4, 3),
(NULL,      NULL,       'McGillicuddy & Sons LLC',  '555-555-5555', '1111 Jupiter St.',     'Boca Raton',       'FL', '33427', NULL,            5, 2),
(NULL,      NULL,       'Proceed Finance',          '555-555-5555', '2700 Fletcher Ave.',   'Lincoln',          'NE', '68512', NULL,            5, 4),
(NULL,      NULL,       'Arp, Swanson & Muldoon',   '555-555-5555', '2424 Hough St.',       'Denver',           'CO', '80014', NULL,            5, 4),
(NULL,      NULL,       'Stugotz Inc',              '555-555-5555', '100 West Ave',         'New York City',    'NY', '10001', NULL,            5, 1);

INSERT INTO consultants (consultant_f_name, consultant_l_name, specialty_id, user_id, img_path) 
VALUES 
('Terry',   'Bolea',    1, 8,  '/images/consultants/hulk_hogan.svg'),
('Mike',    'Ryan',     3, 9,  '/images/consultants/m_w.svg'),
('Mister',  'Zardos',   4, 10, '/images/consultants/m_w.svg'),
('Greg',    'Cote',     2, 11, '/images/consultants/m_w.svg'),
('Robert',  'Bower',    1, 12, '/images/consultants/m_w.svg'),
('Vanessa', 'Smith',    3, 13, '/images/consultants/f_w.svg'),
('Joe',     'Zagacki',  2, 14, '/images/consultants/m_w.svg');

INSERT INTO consultant_ties (consultant_id, specialty_id, territory_id, consultant_start, consultant_end) 
VALUES 
(1, 2, NULL, '2022-02-02', '2023-02-02'),
(2, 1, NULL, '2022-02-02', '2023-02-02'),
(3, 4, 1,    '2022-02-02', '2023-02-02'),
(1, 1, NULL, '2023-02-02', NULL),
(2, 2, NULL, '2023-02-02', NULL),
-- Moving national gov consultant to a region, because we hired one for each region
(3, 4, 4,    '2023-02-02', NULL),
(4, 4, 2,    '2023-02-02', NULL),
(5, 4, 3,    '2023-03-02', NULL),
(6, 4, 5,    '2023-03-02', NULL),
-- Now we have a national tech guy
(7, 3, 1,    '2023-05-27', NULL);

INSERT INTO contacts (contact_title, contact_f_name, contact_l_name, contact_email, contact_primary_phone, contact_secondary_phone) 
VALUES 
('Site Admin',       'Greg',  'Cote',   'cote@gregslobos.com',  '555-555-5555', '555-555-5555'),
('Location Manager', 'Billy', 'Gil',    'bill@marlins.com',     '555-555-5555', '555-555-5555');

INSERT INTO locations (location_name, location_address_one, location_address_two, location_city, location_state, location_zip, location_phone, location_contact_id, territory_id) 
VALUES 
('Default - Main Office',   '1234 Main St.',        NULL,       'Omaha',                            'NE', '68114', '555-555-5555', DEFAULT, 5),
('Bend Conference Center',  '5432 Postgres Ave',    'Ste. 101', 'Bend',                             'OR', '97701', '555-555-5555', DEFAULT, 4),
('101 W',                   '101 W. Ave',           'Ste. 901', 'Chicago',                          'IL', '60007', '555-555-5555', DEFAULT, 5),
('Hilton New York',         '1001 Western St.',     NULL,       'New York',                         'NY', '10001', '555-555-5555', DEFAULT, 2),
('Islands Local',           '70 Oahu Ave',          'Pt. 12',   'Honolulu',                         'HI', '96805', '555-555-5555', DEFAULT, 4),
('LAX Sidepost',            '1 World Way',          NULL,       'Los Angeles',                      'CA', '90045', '555-555-5555', DEFAULT, 4),
('Grosse Pointe Main',      '1212 Main Ln.',        NULL,       'Village of Grosse Pointe Shores',  'MI', '48236', '555-555-5555', DEFAULT, 5),
('Austin Heights',          '6379 Redis Lane',      NULL,       'Austin',                           'TX', '78799', '555-555-5555', 2,       5),
('Princpal Arena',          '98 Santana Ave',       'Ofc. 2',   'Rapid City',                       'SD', '57701', '555-555-5555', DEFAULT, 5),
('New Bluth Home',          '801 Haliburton Dr.',   'Ste. 101', 'Phoenix',                          'AZ', '85007', '555-555-5555', DEFAULT, 4),
('McGillicuddy & Sons',     '300 South Beach Dr.',  NULL,       'Miami',                            'FL', '33109', '555-555-5555', DEFAULT, 3),
('Boston Ceremonial',       '7878 Paul Revere St.', NULL,       'Boston',                           'MA', '02117', '555-555-5555', DEFAULT, 2),
('The Machine Shed',        '1674 Grant St.',       NULL,       'Des Moines',                       'IA', '96805', '555-555-5555', DEFAULT, 5),
('Big Little Building',     '1 Luca Ave',           NULL,       'Reno',                             'NV', '90045', '555-555-5555', DEFAULT, 4),
('The ATL Sky',             '1212 Main Ln.',        NULL,       'Atlanta',                          'GA', '48236', '555-555-5555', DEFAULT, 3),
('Meyer Home',              '771 Benny Dr.',        NULL,       'Dallas',                           'TX', '75001', '555-555-5555', DEFAULT, 5),
('Patton & Smoler',         '0909 Smith Road',      NULL,       'Olympia',                          'WA', '98506', '555-555-5555', DEFAULT, 4),
('Mudra International',     '7878 Homewater St.',   NULL,       'Edina',                            'MN', '55343', '555-555-5555', DEFAULT, 5),
('St. Olaf College',        '1500 St. Olaf Ave.',   NULL,       'Northfield',                       'MN', '55057', '555-555-5555', DEFAULT, 5),
('National Location #1',    '101 National Dr.',     NULL,       'Kansas City',                      'MO', '64109', '555-555-5555', DEFAULT, DEFAULT),
('Thompson Palace',         '1 Mesmer Ave',         'Ste. 222', 'Philadelphia',                     'PA', '19099', '555-555-5555', DEFAULT, 2),
('NOLA Center',             '434 Main Dr.',         NULL,       'New Orleans',                      'LA', '70115', '555-555-5555', DEFAULT, 3),
('MP Heights',              '09 Hermes Way',        NULL,       'Montpelier',                       'VT', '05604', '555-555-5555', 2,       2);

INSERT INTO engagements (rating, text, user_id) 
VALUES 
(7, 'It was a seven.', 1),
(3, 'I give it a 3', 2);

INSERT INTO consults (consultant_id, client_id, location_id, consult_start, consult_end, consult_attachments, notes) 
VALUES 
(1, 4, 2, '2023-09-11 19:10:25', '2023-09-11 19:30:25', ARRAY[2], NULL),
(2, 1, 1, '2022-04-13 12:10:25', '2022-04-13 13:20:11', ARRAY[5], 'An early one with the original folks'),
(1, 2, 1, '2022-04-17 15:10:25', '2022-04-17 15:20:11', NULL, 'Another early one with the original folks'),
(2, 2, 2, '2022-03-17 15:10:25', '2022-03-17 15:20:11', NULL, NULL),
(7, 3, 9, '2023-09-10 12:00:25', '2023-09-10 13:50:11', NULL, 'Rapid City is neat'),
(6, 3, 5, '2023-07-07 12:00:25', '2023-07-07 13:50:11', NULL, 'We went to Hawaii on this one!!'),
(1, 3, 2, '2022-06-19 15:10:25', '2022-06-19 15:20:11', NULL, NULL),
(6, 3, 7, '2023-09-10 12:00:25', '2023-09-10 13:50:11', NULL, 'This is in that one city that is really long'),
(3, 5, 4, '2023-09-13 12:10:25', '2023-09-13 13:20:11', ARRAY[5], 'Arp Swanson and Aribiter met on this one'),
(6, 3, 7, '2023-09-10 12:00:25', '2023-09-10 13:50:11', NULL, 'This is in that one city that is really long'),
(4, 2, 3, '2023-09-14 14:00:00', '2023-09-14 15:11:25', ARRAY[1, 3, 4], 'Hour long session w/ Billy Gil and Tobias. Lots of media!!! See attachments.'),
(2, 2, 1, '2023-09-11 16:00:25', '2023-09-11 16:50:11', NULL, 'Using the Default Address. Location not persisted. Location was at the Clevelander.');

-- audio/flac
INSERT INTO attachments (path, mime_type_id, user_id, channel, created_at) 
VALUES 
('https://upload.wikimedia.org/wikipedia/commons/5/5d/Kuchnia_polska-p243b.png',            1, 3, 'Upload', '2023-09-11 19:10:25-06'),
('https://upload.wikimedia.org/wikipedia/commons/3/3f/Rail_tickets_of_Poland.jpg',          2, 3, 'Upload', '2023-09-11 19:10:25-06'),
('https://upload.wikimedia.org/wikipedia/commons/f/f4/Larynx-HiFi-GAN_speech_sample.wav',   6, 3, 'Upload', '2023-09-11 19:10:25-06'),
('https://upload.wikimedia.org/wikipedia/commons/6/6e/Mindannyian-vagyunk.webm',            9, 3, 'Upload', '2023-09-14 19:16:25-06'),
('https://upload.wikimedia.org/wikipedia/commons/f/f5/Kuchnia_polska-p35b.png',             1, 4, 'Email',  '2023-09-16 16:00:25-06');