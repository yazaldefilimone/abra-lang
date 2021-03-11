import { run } from '../test-utils'

describe('builtin functions', () => {
    test('range', () => {
        const input = `
          val arr = range(0, 4)
          arr
        `;

        expect(run(input)).toEqual([0, 1, 2, 3]);
    });

    test('println', () => {
        // @ts-ignore
        const oldConsole = global.console;
        // @ts-ignore
        global.console = {
            log: jest.fn()
        };

        const input = `
          println("Hello world")
        `;
        run(input);
        // @ts-ignore
        expect(global.console.log).toHaveBeenCalledWith('Hello world');

        // @ts-ignore
        global.console = oldConsole;
    });
});