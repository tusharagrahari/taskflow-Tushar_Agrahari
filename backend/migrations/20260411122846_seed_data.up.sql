INSERT INTO users (id, name, email, password_hash)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'Test User',
    'test@example.com',
    '$2a$12$KpHymLpRLHGrkK50AU5RF.zXvhbRCs3zMGOQWOyaQa6mZTcOK3uAa'
) ON CONFLICT DO NOTHING;

INSERT INTO projects (id, name, description, owner_id)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    'Demo Project',
    'A sample project seeded for testing',
    '00000000-0000-0000-0000-000000000001'
) ON CONFLICT DO NOTHING;

INSERT INTO tasks (id, title, description, status, priority, project_id, creator_id)
VALUES
    (
        '00000000-0000-0000-0000-000000000003',
        'Design database schema',
        'Plan and create all required tables',
        'done',
        'high',
        '00000000-0000-0000-0000-000000000002',
        '00000000-0000-0000-0000-000000000001'
    ),
    (
        '00000000-0000-0000-0000-000000000004',
        'Implement auth endpoints',
        'Register and login with JWT',
        'in_progress',
        'high',
        '00000000-0000-0000-0000-000000000002',
        '00000000-0000-0000-0000-000000000001'
    ),
    (
        '00000000-0000-0000-0000-000000000005',
        'Write README',
        'Document API endpoints and setup instructions',
        'todo',
        'medium',
        '00000000-0000-0000-0000-000000000002',
        '00000000-0000-0000-0000-000000000001'
    )
ON CONFLICT DO NOTHING;
