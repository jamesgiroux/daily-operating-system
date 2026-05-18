#!/usr/bin/env node
/**
 * DailyOS block scaffold CLI.
 *
 * Per L0 Packet C V1.3 §5.1: hybrid invocation pattern.
 * - Flag-only when all required flags are provided (scriptable for codex agents)
 * - Interactive fallback via `prompts` when invoked bare
 *
 * USAGE:
 *   pnpm dailyos:new-block <block-name> [--template simple|typed-display|composite] [--ability <existing-ability-name>] [--keep-partial]
 *
 * Per L0 Packet C V1.3 §5.1 + §6.6: CLI does NOT modify
 * `wp/dailyos/includes/class-dailyos-plugin.php`. The existing
 * `register_blocks()` at `:149-163` uses a glob over blocks subdirectories —
 * dropping a new block.json directory auto-registers.
 *
 * Per V1.3 §5.4: CLI does NOT modify any `.rs` file. Emits paste snippets
 * for the developer to apply manually (5 paste targets per §5.4 V1.3).
 */

'use strict';

import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath( import.meta.url );
const __dirname = path.dirname( __filename );

const REPO_ROOT = path.resolve( __dirname, '..', '..', '..' );
const BLOCKS_DIR = path.resolve( REPO_ROOT, 'wp', 'dailyos', 'blocks' );
const TEMPLATES_DIR = path.resolve( __dirname, 'templates' );

const VALID_TEMPLATES = [ 'simple', 'typed-display', 'composite' ];
const BLOCK_NAME_RE = /^[a-z][a-z0-9-]+$/;
const ABILITY_NAME_RE = /^[a-z][a-z0-9_-]+$/;

const EXIT_OK = 0;
const EXIT_VALIDATION = 1;
const EXIT_PARTIAL = 2;

/**
 * @typedef {Object} ParsedArgs
 * @property {string|null} blockName
 * @property {string} template
 * @property {string|null} ability
 * @property {string|null} newAbility
 * @property {boolean} keepPartial
 * @property {boolean} help
 */

/**
 * Parse argv into ParsedArgs.
 * @param {string[]} argv
 * @returns {ParsedArgs}
 */
function parseArgs( argv ) {
	const args = {
		blockName: null,
		template: 'simple',
		ability: null,
		newAbility: null,
		keepPartial: false,
		help: false,
	};
	const positional = [];
	for ( let i = 0; i < argv.length; i++ ) {
		const a = argv[ i ];
		if ( a === '--help' || a === '-h' ) {
			args.help = true;
		} else if ( a === '--template' ) {
			args.template = argv[ ++i ];
		} else if ( a === '--ability' ) {
			args.ability = argv[ ++i ];
		} else if ( a === '--new-ability' ) {
			args.newAbility = argv[ ++i ];
		} else if ( a === '--keep-partial' ) {
			args.keepPartial = true;
		} else if ( a.startsWith( '--' ) ) {
			console.error( `Unknown flag: ${ a }` );
			process.exit( EXIT_VALIDATION );
		} else {
			positional.push( a );
		}
	}
	if ( positional.length > 0 ) args.blockName = positional[ 0 ];
	return args;
}

function printHelp() {
	console.log( `dailyos:new-block — DailyOS WordPress block scaffold

USAGE
  pnpm dailyos:new-block <block-name>
      [--template simple|typed-display|composite]   default: simple
      [--ability <existing-ability-name>]           link to existing producer
      [--keep-partial]                              don't clean up on partial failure
      [-h|--help]

REQUIRED
  <block-name>     must match ${ BLOCK_NAME_RE.source }

TEMPLATES
  simple           single-payload primitive (Pill, StatusDot, ProvenanceTag)
  typed-display    multiple typed attrs (HealthBadge, Avatar, FreshnessIndicator)
  composite        multi-block composition (AccountOverview-style)

EXIT CODES
  0  success
  1  validation error
  2  partial scaffold (template copy succeeded but a later step failed; cleanup
     manifest printed to stderr; --keep-partial keeps files for debugging)

OUTPUT
  Creates wp/dailyos/blocks/<block-name>/ with templated files.
  Authoring a new ability (the producer side) is hand-authored under
  src-tauri/abilities-runtime/src/abilities/ — automated producer scaffold
  deferred to v1.4.3 W7.

  PRINTS 5 paste snippets for the developer to apply manually:
    [1] BlockType variant → composition.rs:330
    [2] BlockType::type_id() arm → composition.rs:350
    [3] <NAME>_FIELDS const + <name>_rule fn → fallback_projection.rs near :1409/:1415
    [4] rule_for_block_type() arm → fallback_projection.rs:1236
    [5] known_projection_rules() Vec entry → fallback_projection.rs:1250

  CLI does NOT modify class-dailyos-plugin.php (glob auto-registers).
  CLI does NOT modify any .rs file (emits snippets for paste).

DOCS
  L0 Packet C V1.3: .docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md
` );
}

/**
 * @param {string} name
 * @returns {string} PascalCase
 */
function toPascalCase( name ) {
	return name
		.split( /[-_]/ )
		.filter( Boolean )
		.map( ( w ) => w.charAt( 0 ).toUpperCase() + w.slice( 1 ) )
		.join( '' );
}

/**
 * @param {string} name
 * @returns {string} snake_case for PHP function prefix
 */
function toSnakeCase( name ) {
	return name.replace( /-/g, '_' );
}

/**
 * @param {string} name
 * @returns {string} UPPER_SNAKE for Rust const
 */
function toUpperSnake( name ) {
	return name.replace( /-/g, '_' ).toUpperCase();
}

/**
 * Interpolate template placeholders.
 * @param {string} content
 * @param {Record<string, string>} vars
 */
function interpolate( content, vars ) {
	return Object.entries( vars ).reduce(
		( acc, [ key, val ] ) => acc.split( `{{${ key }}}` ).join( val ),
		content
	);
}

/**
 * Recursively copy + interpolate.
 * @param {string} src
 * @param {string} dst
 * @param {Record<string, string>} vars
 */
async function copyTemplate( src, dst, vars ) {
	const entries = await fs.readdir( src, { withFileTypes: true } );
	await fs.mkdir( dst, { recursive: true } );
	for ( const entry of entries ) {
		const srcPath = path.join( src, entry.name );
		const dstPath = path.join( dst, entry.name );
		if ( entry.isDirectory() ) {
			await copyTemplate( srcPath, dstPath, vars );
		} else {
			const content = await fs.readFile( srcPath, 'utf8' );
			await fs.writeFile( dstPath, interpolate( content, vars ) );
		}
	}
}

/**
 * Build the 5-step paste-snippet manifest per V1.3 §5.4.
 * @param {string} blockName
 * @param {string} abilityName
 */
function pasteSnippets( blockName, abilityName ) {
	const BlockType = toPascalCase( abilityName );
	const abilityFnName = toSnakeCase( abilityName );
	const AbilityNameUpper = toUpperSnake( abilityName );
	const abilityKebab = abilityName.replace( /_/g, '-' );

	return `
Manual steps required (paste these into the named files):

[1] Add a BlockType variant to src-tauri/abilities-runtime/src/abilities/composition.rs:330
    (the BlockType enum):

    #[serde(rename = "dailyos/${ abilityKebab }")]
    ${ BlockType },

[2] Add the variant to BlockType::type_id() exhaustive match at
    src-tauri/abilities-runtime/src/abilities/composition.rs:350:

    BlockType::${ BlockType } => "dailyos/${ abilityKebab }",

[3] Add the field-policy const + rule fn IN-FILE to
    src-tauri/abilities-runtime/src/abilities/fallback_projection.rs
    (paste alongside ACCOUNT_OVERVIEW_FIELDS at line ~1409 + account_overview_rule
    at line ~1415):

    const ${ AbilityNameUpper }_FIELDS: &[FieldPolicy] = &[
        text_field("/payload/text", ClaimSensitivity::Internal),
        // TODO: add per-binding field policies. Helpers:
        //   text_field / number_field / bool_field / object_field / array_field
        //   ClaimSensitivity: Public | Internal | Confidential | UserOnly
    ];

    fn ${ abilityFnName }_rule() -> BlockProjectionRule {
        BlockProjectionRule {
            block_type: BlockType::${ BlockType },
            composition_kind: Some("entity_page"),
            type_namespace: Some("dailyos/${ abilityKebab }"),
            render_annotations: &["${ blockName }"],
            fields: ${ AbilityNameUpper }_FIELDS,
            default_trust_band: TrustBand::UseWithCaution,
        }
    }

[4] Add the rule to rule_for_block_type() at
    src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1236
    (match arm):

    BlockType::${ BlockType } => Some(${ abilityFnName }_rule()),

[5] Add the rule to known_projection_rules() at
    src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1250
    (inside the Vec literal):

    ${ abilityFnName }_rule(),

[6] Run the kit integration fixture:
    pnpm dailyos:test-block ${ blockName }
`;
}

async function main() {
	const args = parseArgs( process.argv.slice( 2 ) );

	if ( args.help || ! args.blockName ) {
		printHelp();
		process.exit( args.help ? EXIT_OK : EXIT_VALIDATION );
	}

	// 1. Validate block name.
	if ( ! BLOCK_NAME_RE.test( args.blockName ) ) {
		console.error(
			`error: block-name "${ args.blockName }" must match ${ BLOCK_NAME_RE.source }`
		);
		process.exit( EXIT_VALIDATION );
	}

	// 2. Validate template.
	if ( ! VALID_TEMPLATES.includes( args.template ) ) {
		console.error(
			`error: --template must be one of ${ VALID_TEMPLATES.join( '|' ) }; got "${ args.template }"`
		);
		process.exit( EXIT_VALIDATION );
	}

	// 3. Enumerate existing blocks; check name collision.
	const existing = await fs
		.readdir( BLOCKS_DIR, { withFileTypes: true } )
		.then( ( entries ) => entries.filter( ( e ) => e.isDirectory() ).map( ( e ) => e.name ) )
		.catch( () => [] );
	if ( existing.includes( args.blockName ) ) {
		console.error(
			`error: block "${ args.blockName }" already exists at wp/dailyos/blocks/${ args.blockName }/`
		);
		process.exit( EXIT_VALIDATION );
	}

	// 4. Validate ability flag.
	if ( args.newAbility ) {
		console.error(
			'error: --new-ability is not implemented in v1.4.3 W1 (producer scaffold deferred to W7).\n' +
			'       Use --ability <existing-ability-name> to link to an already-authored ability,\n' +
			'       or hand-author a new ability under src-tauri/abilities-runtime/src/abilities/\n' +
			'       modeled on a small existing file (e.g. tracer.rs).'
		);
		process.exit( EXIT_VALIDATION );
	}
	if ( args.ability && ! ABILITY_NAME_RE.test( args.ability ) ) {
		console.error( `error: --ability "${ args.ability }" must match ${ ABILITY_NAME_RE.source }` );
		process.exit( EXIT_VALIDATION );
	}

	const abilityName = args.ability || args.blockName.replace( /-/g, '_' );
	const vars = {
		BLOCK_NAME: args.blockName,
		BLOCK_TITLE: args.blockName
			.split( '-' )
			.map( ( w ) => w.charAt( 0 ).toUpperCase() + w.slice( 1 ) )
			.join( ' ' ),
		BLOCK_TITLE_PASCAL: toPascalCase( args.blockName ),
		BLOCK_DESCRIPTION: `DailyOS ${ args.blockName } block — render-only.`,
		ABILITY_NAME: abilityName,
		ABILITY_NAME_KEBAB: abilityName.replace( /_/g, '-' ),
		ABILITY_NAME_UPPER: toUpperSnake( abilityName ),
		PHP_FUNCTION_PREFIX: toSnakeCase( args.blockName ),
		BlockType: toPascalCase( abilityName ),
		ability_fn_name: toSnakeCase( abilityName ),
		ability_name: toSnakeCase( abilityName ),
	};

	const templateSrc = path.join( TEMPLATES_DIR, args.template );
	const blockDst = path.join( BLOCKS_DIR, args.blockName );

	try {
		await copyTemplate( templateSrc, blockDst, vars );
	} catch ( err ) {
		console.error( `error: template copy failed: ${ err.message }` );
		if ( ! args.keepPartial ) {
			await fs.rm( blockDst, { recursive: true, force: true } ).catch( () => {} );
		}
		process.exit( EXIT_PARTIAL );
	}

	console.log( `Block scaffold created at wp/dailyos/blocks/${ args.blockName }/` );

	if ( args.ability ) {
		console.log( pasteSnippets( args.blockName, abilityName ) );
	} else {
		console.log(
			`\n(no --ability: skipping projection paste snippets;\n` +
			` block is render-only over an existing ability or a substrate composition.\n` +
			` if you need a NEW ability, hand-author one under src-tauri/abilities-runtime/src/abilities/\n` +
			` modeled on a small existing ability — producer scaffold automation deferred to v1.4.3 W7.)`
		);
	}

	console.log( `\nNext: apply paste snippets, drop a Rust integration fixture at` );
	console.log( `  src-tauri/abilities-runtime/tests/fixtures/${ args.blockName.replace( /-/g, '_' ) }_integration_fixture.rs,` );
	console.log( `  then run:` );
	console.log( `  cargo test -p abilities-runtime --test block_kit_integration_harness` );

	process.exit( EXIT_OK );
}

main().catch( ( err ) => {
	console.error( `error: ${ err.message }` );
	console.error( err.stack );
	process.exit( EXIT_PARTIAL );
} );
