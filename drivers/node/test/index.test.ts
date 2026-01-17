import { PrimusDB } from '../src/index';

describe('PrimusDB Node.js Driver', () => {
  let db: PrimusDB;

  beforeAll(async () => {
    db = new PrimusDB('localhost', 8080);
    // Note: Tests assume PrimusDB server is running on localhost:8080
    // In a real test environment, you might want to start a test server
  });

  afterAll(async () => {
    // Cleanup if needed
  });

  test('should create PrimusDB instance', () => {
    expect(db).toBeInstanceOf(PrimusDB);
    expect(db.isConnected()).toBe(false);
  });

  test('should have correct configuration', () => {
    const db2 = new PrimusDB('example.com', 9090, { timeout: 5000 });
    expect(db2).toBeInstanceOf(PrimusDB);
  });

  // Note: Integration tests below require a running PrimusDB server
  // Uncomment and modify as needed for actual testing

  /*
  test('should connect to server', async () => {
    await expect(db.connect()).resolves.not.toThrow();
    expect(db.isConnected()).toBe(true);
  });

  test('should perform health check', async () => {
    const health = await db.health();
    expect(health).toBeDefined();
  });

  test('should create table', async () => {
    await expect(db.createTable('document', 'test_users', {
      name: 'string',
      email: 'string'
    })).resolves.not.toThrow();
  });

  test('should insert and select data', async () => {
    const insertCount = await db.insert('document', 'test_users', {
      name: 'Test User',
      email: 'test@example.com'
    });
    expect(insertCount).toBe(1);

    const results = await db.select('document', 'test_users');
    expect(Array.isArray(results)).toBe(true);
  });

  test('should disconnect', async () => {
    await expect(db.disconnect()).resolves.not.toThrow();
    expect(db.isConnected()).toBe(false);
  });
  */
});