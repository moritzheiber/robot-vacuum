-- Add migration script here
CREATE TABLE executions (
    /*  For sqlite we need to get rid of AUTO_INCREMENT since it doesn't
        exist in their dialect (instead it's AUTOINCREMENT).

        PRIMARY KEY is also enough since it gets automatically incremented

    id INTEGER PRIMARY KEY, */
    id SERIAL PRIMARY KEY,
    
    /*  sqlite also only has the TIMESTAMP type

    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL, */
    timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    commands int,
    result int,
    duration float
); 
