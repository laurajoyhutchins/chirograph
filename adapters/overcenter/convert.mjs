import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const CATALOG_SCHEMA = 'contract-evidence-catalog-v1';
const CLASSIFICATION_SCHEMA = 'contract-evidence-classifications-v1';
const CHIROGRAPH_SCHEMA = 'chirograph-evidence-v1';

const RELATION_KIND = Object.freeze({
  consumes: 'depends_on',
  produces: 'defines',
  'persists-as': 'projects',
  'derives-from': 'depends_on',
  'verified-by': 'validates',
  'compatibility-for': 'depends_on',
});

function fail(message) {
  throw new Error(message);
}

function record(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function text(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

function candidateLocator(candidate) {
  const path = candidate?.source_location?.path;
  if (!text(path)) fail(`candidate ${candidate?.source_identity ?? '<unknown>'} is missing source_location.path`);
  const anchor = candidate.source_location.anchor;
  return text(anchor) ? `${path}#${anchor}` : path;
}

function representationKind(sourceKind) {
  switch (sourceKind) {
    case 'postgres':
    case 'mcp':
    case 'semantic-command':
      return 'schema';
    case 'typescript':
      return 'type_definition';
    case 'javascript':
      return 'source_code';
    default:
      return 'other';
  }
}

function classificationObservationId(sourceIdentity) {
  return `classification:${sourceIdentity}`;
}

function candidateSourceId(sourceIdentity) {
  return `source:${sourceIdentity}`;
}

function node(kind, id) {
  return { kind, id };
}

function sortById(items) {
  return items.sort((left, right) => left.id.localeCompare(right.id));
}

function sortRelations(items) {
  return items.sort((left, right) =>
    `${left.kind}:${left.from.kind}:${left.from.id}:${left.to.kind}:${left.to.id}`
      .localeCompare(`${right.kind}:${right.from.kind}:${right.from.id}:${right.to.kind}:${right.to.id}`));
}

export function convertOvercenterEvidence(catalog, classifications) {
  if (!record(catalog) || catalog.schema !== CATALOG_SCHEMA || !Array.isArray(catalog.candidates)) {
    fail(`expected ${CATALOG_SCHEMA}`);
  }
  if (!record(classifications) || classifications.schema !== CLASSIFICATION_SCHEMA || !record(classifications.candidates)) {
    fail(`expected ${CLASSIFICATION_SCHEMA}`);
  }

  const candidates = new Map();
  for (const candidate of catalog.candidates) {
    if (!record(candidate) || !text(candidate.source_identity) || !text(candidate.source_kind)) {
      fail('catalog candidate must define source_identity and source_kind');
    }
    if (candidates.has(candidate.source_identity)) fail(`duplicate candidate ${candidate.source_identity}`);
    candidates.set(candidate.source_identity, candidate);
  }

  const entries = Object.entries(classifications.candidates).sort(([left], [right]) => left.localeCompare(right));
  const logicalBySource = new Map();
  const authorityByContract = new Map();

  for (const [sourceIdentity, classification] of entries) {
    if (!candidates.has(sourceIdentity)) fail(`classification references missing candidate ${sourceIdentity}`);
    if (!record(classification) || !text(classification.significance)) fail(`invalid classification ${sourceIdentity}`);

    if (classification.significance === 'projection') {
      if (!text(classification.projection_of) || classification.logical_contract !== undefined) {
        fail(`projection ${sourceIdentity} must define projection_of and no logical_contract`);
      }
      logicalBySource.set(sourceIdentity, classification.projection_of);
      continue;
    }

    if (!text(classification.logical_contract)) fail(`classification ${sourceIdentity} requires logical_contract`);
    if (authorityByContract.has(classification.logical_contract)) {
      fail(`multiple Overcenter authorities for ${classification.logical_contract}`);
    }
    logicalBySource.set(sourceIdentity, classification.logical_contract);
    authorityByContract.set(classification.logical_contract, sourceIdentity);
  }

  for (const [sourceIdentity, classification] of entries) {
    if (classification.significance === 'projection' && !authorityByContract.has(classification.projection_of)) {
      fail(`projection ${sourceIdentity} references missing logical contract ${classification.projection_of}`);
    }
    for (const relationship of classification.relationships ?? []) {
      if (!record(relationship) || !RELATION_KIND[relationship.kind] || !text(relationship.target)) {
        fail(`invalid relationship on ${sourceIdentity}`);
      }
      if (!authorityByContract.has(relationship.target)) {
        fail(`relationship on ${sourceIdentity} references missing logical contract ${relationship.target}`);
      }
    }
  }

  const sources = [{
    id: 'overcenter:classifications',
    kind: 'repository',
    locator: '.contract-evidence/classifications.json',
  }];
  const representations = [];
  const observations = [];
  const authorityClaims = [];
  const relations = [];

  for (const [sourceIdentity, classification] of entries) {
    const candidate = candidates.get(sourceIdentity);
    const logicalContract = logicalBySource.get(sourceIdentity);
    const observation = classificationObservationId(sourceIdentity);
    const locator = candidateLocator(candidate);

    sources.push({
      id: candidateSourceId(sourceIdentity),
      kind: 'repository',
      locator,
    });
    representations.push({
      id: sourceIdentity,
      contract: logicalContract,
      source: candidateSourceId(sourceIdentity),
      kind: representationKind(candidate.source_kind),
      locator,
      facets: ['structural'],
    });
    observations.push({
      id: observation,
      source: 'overcenter:classifications',
      revision: { kind: 'unknown' },
      locator: sourceIdentity,
      fact: classification.significance === 'projection'
        ? `${sourceIdentity} is explicitly classified as a projection of ${logicalContract}`
        : `${sourceIdentity} is explicitly classified as the authority for ${logicalContract}`,
    });

    if (classification.significance === 'projection') {
      relations.push({
        from: node('representation', sourceIdentity),
        to: node('contract', logicalContract),
        kind: 'projects',
        basis: [observation],
      });
    } else {
      authorityClaims.push({
        contract: logicalContract,
        representation: sourceIdentity,
        facet: 'structural',
        basis: 'explicit_declaration',
        evidence: [observation],
      });
      for (const relationship of classification.relationships ?? []) {
        relations.push({
          from: node('contract', logicalContract),
          to: node('contract', relationship.target),
          kind: RELATION_KIND[relationship.kind],
          basis: [observation],
        });
      }
    }
  }

  const contracts = [...authorityByContract.keys()].map((id) => ({
    id,
    name: id,
    facets: ['structural'],
  }));

  return {
    schema: CHIROGRAPH_SCHEMA,
    sources: sortById(sources),
    contracts: sortById(contracts),
    representations: sortById(representations),
    observations: sortById(observations),
    clauses: [],
    clause_assertions: [],
    relations: sortRelations(relations),
    authority_claims: authorityClaims.sort((left, right) =>
      `${left.contract}:${left.representation}`.localeCompare(`${right.contract}:${right.representation}`)),
  };
}

async function main(argv) {
  if (argv.length !== 2) fail('usage: node convert.mjs <catalog.json> <classifications.json>');
  const [catalogPath, classificationsPath] = argv;
  const [catalog, classifications] = await Promise.all([
    readFile(catalogPath, 'utf8').then(JSON.parse),
    readFile(classificationsPath, 'utf8').then(JSON.parse),
  ]);
  process.stdout.write(`${JSON.stringify(convertOvercenterEvidence(catalog, classifications), null, 2)}\n`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
