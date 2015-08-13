# kit

Kit really, really wants to be an operating system. Right now it's mostly just a
kernel, but some really interesting things are in the works.

It currently only supports x86_64, but may be ported to other architectures in
the future.

It's written in a mix of C and Rust, with ongoing efforts to remove all of the C
from the kernel since Rust is a superior language. The userland will have native
support libraries for both C and Rust, but currently only really supports C.

## Building / Running

You'll need to have the following installed:

- GRUB 2
- cdrkit
- Ruby
- Rust nightly (1.4)
- clang
- QEMU

After that, just run `make`. If all goes well, you should be able to run `make
run-qemu` and see Kit running.

The commands you can run are all listed in `build/system/bin`, and here's what
they do:

- **echo**: prints arguments to the screen.
- **true**: exits with status 0.
- **false**: exits with status 1.
- **key**: tests keyboard input. Press control-D to exit.
- **poke_null**: crashes the system currently, but really should only crash the
  process.
- **shell**: what you're in right now. There's no way to get out of a shell
  currently.
- **yield**: argument should be a number, which is a multiplier for the number
  of cycles to spin and do nothing between yielding to the scheduler. This was
  originally used to test the cooperative multitasking, but now multitasking is
  preemptive. Try `yield 30 & key` to see that you can actually still type while
  `yield` is using all of the processor time. That wasn't the case before.

## What's going on?

- Improving the C standard library coverage to be able to port Lua (branch:
  [topic/lua](https://github.com/devyn/kit/branch/topic/lua))

## What's planned for the future?

- Replace `bin/shell` with a Lua interpreter.
- Complete syscall overhaul, with a generic IPC interface and no C strings.
- IPC/host model. Each process, except the root host, will have a **host**
  process defined that is expected to handle configuration at the current scope.
- Application model, with a protocol probably resembling BSON, because I think
  it will be really cool.
- No args. No pipes. No stdin/stdout. This ain't UNIX. Processes only
  communicate with their host, and whatever other processes their host has
  granted them permission to talk to.
- Of course, C compatibility is nice, so we'll put a layer over all of this that
  allows C applications to function as intended.

    1> apps.editor:start()
    2> apps.editor:open(recipes["ginger stir fry"])

And then you're in the editor. Sound fun? Haven't really figured out the
specifics, but this is the direction I want to go in.
