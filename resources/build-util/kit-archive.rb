#!/usr/bin/env ruby

abort "Usage: #$0 <system-dir> <files>" unless ARGV.size >= 2 and
                                        File.directory?(ARGV[0])

NUL = "\0".b

system_dir = File.expand_path(ARGV[0])
files      = ARGV[1..-1]

Entry = Struct.new(:name, :offset, :checksum, :size)

entries = []

warn "\e[1;34m# \e[0;1mBuilding entry list\e[0m"

offset = 0

files.each do |file|
  path = File.join(system_dir, file)

  abort " file not found: #{file}" unless File.file?(path)

  checksum = 0
  size = nil

  File.open(path, "rb") do |f|
    until f.eof?
      checksum ^= f.read(8).ljust(8, NUL).unpack("Q<")[0]
    end
    size = f.size
  end

  entries << Entry.new(file, offset, checksum, size)

  offset += size
end

# Calculate the size of the header and offset the entries

header_size = 16 + entries.map { |e| 32 + e.name.bytesize }.inject(:+)

entries.each { |entry| entry.offset += header_size }

# Write the header

warn "\e[1;34m# \e[0;1mWriting header\e[0m"

print "kit AR01"

print [entries.size].pack("Q<") # u64-LE

entries.each do |entry|
  print [entry.offset, entry.size, entry.checksum,
         entry.name.bytesize, entry.name].pack("Q<Q<Q<Q<a*")

  $stderr << ("  %016x - %016x       %s (%016x)\n" %
              [entry.offset, entry.offset + entry.size, entry.name,
               entry.checksum])
end

# Now start writing entry data

warn "\e[1;34m# \e[0;1mWriting entries\e[0m"

offset = header_size

entries.each do |entry|
  if offset < entry.offset
    # Pad to the entry's beginning
    print "\0" * (entry.offset - offset)

    offset = entry.offset
  end

  $stderr << ("  %-40s" % entry.name)
  $stderr.flush

  written = 0

  File.open(File.join(system_dir, entry.name), "rb") do |file|
    until file.eof?
      s = file.read(4096)
      $stdout << s
      written += s.length
    end
  end

  if written == entry.size
    $stderr << "[ \e[32mOK\e[0m ]\n"
  else
    $stderr << "[ \e[31mERR\e[0m]\n"
    exit 1
  end

  offset += written
end

warn "\e[1;34m# \e[0;1mEnd of archive\e[0m"
warn "  #{offset} bytes written."
