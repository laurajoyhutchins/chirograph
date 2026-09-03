import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { convertOvercenterEvidence } from './convert.mjs';

const fixture = async (name) => JSON.parse(await readFile(new URL(`./fixtures/${name}`, import.meta.url), 'utf8'));

const byId = (items, id) => items.find((item) => item.id === id);

await test('converts Overcenter authority, projection, and relationships without inventing clauses', async () => {
  const evidence = convertOvercenterEvidence(
    await fixture('catalog.json'),
    await fixture('classifications.json'),
  );

  assert.equal(evidence.schema, 'chirograph-evidence-v1');
  assert.deepEqual(evidence.clauses, []);
  assert.deepEqual(evidence.clause_assertions, []);

  assert.deepEqual(byId(evidence.contracts, 'example.model').facets, ['structural']);
  assert.deepEqual(byId(evidence.contracts, 'example.store').facets, ['structural']);

  const modelAuthority = evidence.authority_claims.find((claim) => claim.contract === 'example.model');
  assert.equal(modelAuthority.representation, 'typescript:src/model.ts#Example');
  assert.equal(modelAuthority.facet, 'structural');
  assert.equal(modelAuthority.basis, 'explicit_declaration');

  const projection = evidence.relations.find((relation) =>
    relation.kind === 'projects' && relation.from.id === 'postgres:public.example#table');
  assert.deepEqual(projection.to, { kind: 'contract', id: 'example.model' });

  const persistence = evidence.relations.find((relation) =>
    relation.kind === 'projects' && relation.from.id === 'example.store');
  assert.deepEqual(persistence.to, { kind: 'contract', id: 'example.model' });
});
