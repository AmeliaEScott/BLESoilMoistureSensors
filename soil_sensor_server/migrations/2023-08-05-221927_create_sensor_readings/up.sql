CREATE TABLE sensor_readings (
  id SERIAL PRIMARY KEY,
  sensor_id INTEGER NOT NULL,
  hardware_address MACADDR NOT NULL,
  sequence INTEGER NOT NULL,
  moisture INTEGER NOT NULL,
  temperature FLOAT NOT NULL,
  capacitor_voltage FLOAT NOT NULL,
  time TIMESTAMP WITH TIME ZONE NOT NULL
)