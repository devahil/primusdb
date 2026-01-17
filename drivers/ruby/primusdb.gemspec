# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = 'primusdb'
  spec.version       = '0.1.0'
  spec.authors       = ['PrimusDB Team']
  spec.email         = ['team@primusdb.com']

  spec.summary       = 'Ruby driver for PrimusDB hybrid database engine'
  spec.description   = 'High-performance Ruby client for PrimusDB supporting all storage engines and advanced features'
  spec.homepage      = 'https://primusdb.com'
  spec.license       = 'MIT'
  spec.required_ruby_version = Gem::Requirement.new('>= 2.7.0')

  spec.metadata['homepage_uri'] = spec.homepage
  spec.metadata['source_code_uri'] = 'https://github.com/primusdb/primusdb'
  spec.metadata['changelog_uri'] = 'https://github.com/primusdb/primusdb/blob/main/CHANGELOG.md'

  # Specify which files should be added to the gem when it is released.
  spec.files = Dir.glob('lib/**/*') + Dir.glob('ext/**/*') + %w[README.md LICENSE]
  spec.bindir        = 'exe'
  spec.executables   = spec.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  spec.require_paths = ['lib']

  # Runtime dependencies
  spec.add_runtime_dependency 'concurrent-ruby', '~> 1.1'
  spec.add_runtime_dependency 'faraday', '~> 2.0'
  spec.add_runtime_dependency 'faraday-multipart', '~> 1.0'

  # Development dependencies
  spec.add_development_dependency 'bundler', '~> 2.0'
  spec.add_development_dependency 'rake', '~> 13.0'
  spec.add_development_dependency 'rspec', '~> 3.0'
  spec.add_development_dependency 'rubocop', '~> 1.0'
  spec.add_development_dependency 'yard', '~> 0.9'
end
