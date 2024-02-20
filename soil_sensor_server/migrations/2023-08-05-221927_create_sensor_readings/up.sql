CREATE TABLE sensors (
    id SERIAL PRIMARY KEY,
    display_id INTEGER,
    hardware_address MACADDR NOT NULL,
    description TEXT
);

CREATE TABLE measurements (
  id SERIAL PRIMARY KEY,
  sensor_id INTEGER NOT NULL REFERENCES sensors (id),
  sequence INTEGER NOT NULL,
  moisture INTEGER NOT NULL,
  temperature FLOAT NOT NULL,
  capacitor_voltage FLOAT NOT NULL,
  time TIMESTAMP WITH TIME ZONE NOT NULL
);