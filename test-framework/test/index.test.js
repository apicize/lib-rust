const runTestSuite = require('../index')

test('runsTestSuite', () => {
    const results = runTestSuite({}, {}, {}, {}, {}, 0, () => {
        describe('test', () => {
            tag('foo')
            it('should be ok', () => {
                expect(1).to.equal(1)
            })
        })
    })
    expect(results).to.be.an('string')
})


