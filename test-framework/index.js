// Stub enough of process to make browserify's util happy...
process = { env: {} }

const chai = require('chai');
const format = require('util').format;

const jpp = require('jsonpath-plus');
// const xmldom = require('@xmldom/xmldom');

let testOffset = 0;
let logs = []

function fmtMinSec(value, subZero = null) {
    if (value === 0 && subZero) {
        return subZero
    }
    const m = Math.floor(value / 60000)
    value -= m * 60000
    const s = Math.floor(value / 1000)
    value -= s * 1000
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}${(0.1).toString()[1]}${value.toString().padEnd(3, '0')}`
}


function appendLog(type, message, ...optionalParams) {
    const timestamp = fmtMinSec(Date.now() - testOffset)
    logs.push(`${timestamp} [${type}] ${format(message, ...optionalParams)}`)
}

/******************************************************************
 * Global variables exposed to test runner
 ******************************************************************/

request = {};
response = {};
variables = {};
scenario = {};
data = {};
$ = {};
outputVars = {};

assert = chai.assert;
expect = chai.expect;
should = chai.should;

jsonpath = jpp.JSONPath;
// xpath = require('xpath');
// dom = xmldom.DOMParser;

// Helper function to jsonpath-plus
function jpath(param) {
    if (typeof param === 'object') {
        return jpp.JSONPath({ ...param, json: this })
    } else if (typeof param === 'string') {
        return jpp.JSONPath({ path: param, json: this })
    } else {
        throw new Error('Argument for jp must be either a JSON path (string) or named parameters')
    }
}

Object.prototype.jp = jpath
Array.prototype.jp = jpath
String.prototype.jp = jpath
Number.prototype.jp = jpath

console = {
    log: (msg, ...args) => appendLog('log', msg, ...args),
    info: (msg, ...args) => appendLog('info', msg, ...args),
    warn: (msg, ...args) => appendLog('warn', msg, ...args),
    error: (msg, ...args) => appendLog('error', msg, ...args),
    trace: (msg, ...args) => appendLog('trace', msg, ...args),
    debug: (msg, ...args) => appendLog('debug', msg, ...args),
};

BodyType = {
    JSON: 'JSON',
    XML: 'XML',
    Text: 'Text',
    Form: 'Form',
    Binary: 'Binary'
}

function generateTag(arg) {
    let tagName = undefined

    switch (typeof (arg)) {
        case 'boolean':
        case 'undefined':
        case 'array':
        case 'function':
            throw new Error('Invalid parameter for "tag"')
        default:
            if (arg !== null) {
                const n = `${arg}`
                if (n.length > 0) {
                    tagName = n
                }
            }
            break
    }

    if (tagName) {
        const props = tagName.matchAll(/\{\{(.*?)\}\}/g)
        for(const [match, propName] of props) {
            const value = $[propName]
            if (! value) {
                throw new Error(`"${propName}" is not available for use in a tag`)
            }
            tagName = tagName.replaceAll(match, $[propName] ? `${value}` : '')
        }
    }

    return tagName
}

class Scenario {
    constructor(name) {
        this.type = 'Scenario'
        this.name = name
        this.tag = undefined
        this.success = true
        this.children = []
        this.testCount = 0
        this.testFailCount = 0
    }
}

class Behavior {
    constructor(name) {
        this.type = 'Behavior'
        this.name = name
        this.tag = undefined
        this.success = true
        this.testCount = 0
        this.testFailCount = 0
    }

    succeed() {
        this.success = true
        this.testCount = 1
    }

    fail(e) {
        this.success = false
        this.testCount = 1
        this.testFailCount = 1
        this.error = e
    }
}

class Context {
    constructor() {
        this.results = []
        this.currentResult = null
        this.inScenario = false
        this.inBehavior = false
    }

    push(scenarioOrbehavior) {
        if (this.currentResult == null) {
            this.results.push(scenarioOrbehavior)
        } else {
            scenarioOrbehavior.parent = this.currentResult
            if (scenarioOrbehavior.parent.children) {
                scenarioOrbehavior.parent.children.push(scenarioOrbehavior)
            } else {
                scenarioOrbehavior.parent.children = [scenarioOrbehavior]
            }
        }
        this.currentResult = scenarioOrbehavior

    }

    pop() {
        const current = this.currentResult
        if (!current) {
            return
        }
        if (logs && logs.length > 0) {
            current.logs = logs
            logs = []
        }
        if (current.parent) {
            if (current.parent.tag) {
                current.tag = current.parent.tag + (current.tag ? '.' + current.tag : '')
            }
            current.parent.success = current.success && current.parent.success
            current.parent.testCount += current.testCount
            current.parent.testFailCount += current.testFailCount
            this.currentResult = current.parent
            current.parent = undefined
        } else {
            this.currentResult = null
        }
    }

    enterScenario(name) {
        const scenario = new Scenario(name)
        this.push(scenario)
        return scenario
    }

    exitScenario() {
        this.pop()
    }

    enterBehavior(name) {
        if (this.currentResult?.type !== 'Scenario') {
            throw new Error('"it" must be called from within a "describe" block')
        }
        const behavior = new Behavior(name)
        this.push(behavior)
        return behavior
    }

    exitBehavior() {
        this.pop()
    }
}

let context = new Context()

describe = (name, run) => {
    context.enterScenario(name)
    try {
        run()
    } finally {
        context.exitScenario()
    }
}

it = (name, run) => {
    const behavior = context.enterBehavior(name)
    try {
        run();
        behavior.succeed()
    } catch (e) {
        behavior.fail(e.message)
    } finally {
        context.exitBehavior()
    }
}

tag = (arg) => {
    context.currentResult.tag = generateTag(arg)
}

output = (name, value) => {
    switch (typeof value) {
        case 'function':
            throw new Error('Functions cannot be output')
        case 'symbol':
            throw new Error('Symbols cannot be output')
        case 'undefined':
            delete outputVars[name]
            break
        default:
            outputVars[name] = value
            break
    }
}

runTestSuite = (request1, response1, variables1, data1, output1, testOffset1, testSuite) => {
    request = request1
    response = response1
    scenario = variables1 ?? {}
    data = data1 ?? {}
    outputVars = output1 ?? {}

    $ = { ...outputVars, ...scenario, ...data }
    variables = $ // retain variables for some level of backward compatibility

    testOffset = testOffset1
    testSuite()

    return JSON.stringify({
        results: context.results,
        output: outputVars
    })
};

module.exports = runTestSuite